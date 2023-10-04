use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use serde_json::Map;

pub type Document = Map<String, serde_json::Value>;

type Field = String;

#[derive(Default, Debug)]
pub struct FieldAccumulator {
    well_defined: HashMap<Field, Settings>,
    unknown: HashMap<Field, HashSet<Value>>,
}

#[derive(Debug, Clone)]
struct Value {
    value: serde_json::Value,
    entropy: f64,
    count: usize,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.value {
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
                0.hash(state)
            }
            serde_json::Value::String(s) => s.hash(state),
            serde_json::Value::Array(arr) => {
                1.hash(state);
                for el in arr {
                    match el {
                        serde_json::Value::String(s) => {
                            s.hash(state);
                        }
                        serde_json::Value::Null
                        | serde_json::Value::Bool(_)
                        | serde_json::Value::Number(_)
                        | serde_json::Value::Array(_)
                        | serde_json::Value::Object(_) => (),
                    }
                }
            }
            serde_json::Value::Object(_) => (),
        }
    }
}

#[derive(Debug, Clone)]
struct Settings(Vec<Setting>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Setting {
    Searchable,
    Filterable,
    Sortable,
}

impl FieldAccumulator {
    pub fn new() -> FieldAccumulator {
        FieldAccumulator::default()
    }

    pub fn push(&mut self, document: &Document) {
        let document = flatten_serde_json::flatten(document);
        for (key, value) in document {
            let entry = self.unknown.entry(key).or_default();
            let mut to_insert = Value {
                value: value.clone(),
                entropy: 0.,
                count: 1,
            };
            if let Some(Value { count, entropy, .. }) = entry.get(&to_insert) {
                to_insert.count += count;
                to_insert.entropy = *entropy;
            } else {
                match value {
                    serde_json::Value::String(s) => to_insert.entropy = entropy_of(s.chars()),
                    // in case of an array we're going to average all the entropy maybe?
                    serde_json::Value::Array(arr) => {
                        let mut total = 0;
                        for value in arr {
                            if let serde_json::Value::String(s) = value {
                                total += 1;
                                to_insert.entropy += entropy_of(s.chars())
                            }
                        }
                        to_insert.entropy /= total as f64;
                    }
                    _ => (),
                }
            };
            entry.insert(to_insert);
        }
    }

    pub fn finish(mut self) -> FinalSettings {
        for (field, values) in &self.unknown {
            let total: usize = values.iter().map(|value| value.count).sum();

            let mut searchable_score: isize = 0;
            let mut filterable_score: isize = 0;
            let mut sortable_score: isize = 0;

            let mut probs: Vec<f64> = Vec::new();
            let mut per_field_values: HashMap<String, usize> = Default::default();
            let mut nb_values: usize = 0;
            let mut avg_inner_field_entropy = 0.;
            let mut avg_inner_field_entropy_total = 0;

            for value in values {
                if value.entropy > 0. {
                    avg_inner_field_entropy += value.entropy;
                    avg_inner_field_entropy_total += 1;
                }
                probs.push(value.count as f64 / total as f64);

                match &value.value {
                    serde_json::Value::Null => (),
                    serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
                        filterable_score += value.count as isize;
                        sortable_score += value.count as isize;
                    }
                    serde_json::Value::Array(array) => {
                        // TODO: sorting in an array seems like a bad ideas right?
                        sortable_score -= value.count as isize;
                        for element in array {
                            match element {
                                serde_json::Value::Number(_) => filterable_score += 1,
                                serde_json::Value::String(s) => {
                                    *per_field_values.entry(s.clone()).or_default() += 1;
                                    nb_values += 1;

                                    if look_like_id(s) {
                                        searchable_score -= value.count as isize;
                                        filterable_score += value.count as isize;
                                    } else {
                                        searchable_score += value.count as isize;
                                        // size of an uuid-v4
                                        if s.len() > 36 {
                                            searchable_score += value.count as isize;
                                            filterable_score -= value.count as isize * 2;
                                            sortable_score -= value.count as isize * 2;
                                        }

                                        let size = s.chars().count();
                                        if size > 100
                                            && s.chars().filter(|c| c.is_alphabetic()).count()
                                                as f64
                                                / size as f64
                                                * 100.
                                                > 80.
                                        {
                                            searchable_score += value.count as isize * 2;
                                        }
                                    }

                                    for regex in ONLY_DISPLAY.iter() {
                                        if regex.is_match(s) {
                                            searchable_score -= 10 * value.count as isize;
                                            filterable_score -= value.count as isize;
                                        }
                                    }
                                    for regex in FILTER_BUT_NOT_SEARCH.iter() {
                                        if regex.is_match(s) {
                                            filterable_score += value.count as isize;
                                            searchable_score -= 10 * value.count as isize;
                                        }
                                    }
                                    for regex in ONLY_SORT_AND_FILTER.iter() {
                                        if regex.is_match(s) {
                                            filterable_score += value.count as isize;
                                            searchable_score -= 10 * value.count as isize;
                                        }
                                    }
                                }
                                // This has been flattened and should be ignored.
                                serde_json::Value::Array(_) => (),
                                // TODO: That seems useless right?
                                serde_json::Value::Bool(_) => (),
                                // null and object can be ignored
                                _ => (),
                            }
                        }
                    }
                    serde_json::Value::String(s) => {
                        if look_like_id(s) {
                            searchable_score -= value.count as isize;
                            filterable_score += value.count as isize;
                        } else {
                            searchable_score += value.count as isize;
                            // size of an uuid-v4
                            if s.len() > 36 {
                                searchable_score += value.count as isize;
                                filterable_score -= value.count as isize * 2;
                                sortable_score -= value.count as isize * 2;
                            }

                            let size = s.chars().count();
                            if size > 100
                                && s.chars().filter(|c| c.is_alphabetic()).count() as f64
                                    / size as f64
                                    * 100.
                                    > 80.
                            {
                                searchable_score += value.count as isize * 2;
                            }
                        }
                        filterable_score += 1;
                        *per_field_values.entry(s.clone()).or_default() += 1;
                        nb_values += 1;

                        for regex in ONLY_DISPLAY.iter() {
                            if regex.is_match(s) {
                                searchable_score -= 10;
                                // TODO: should we decrement the score of these two one.
                                filterable_score -= 1;
                                sortable_score -= 1;
                            }
                        }
                        for regex in FILTER_BUT_NOT_SEARCH.iter() {
                            if regex.is_match(s) {
                                searchable_score -= 10;
                                filterable_score += 1;
                            }
                        }
                        for regex in ONLY_SORT_AND_FILTER.iter() {
                            if regex.is_match(s) {
                                searchable_score -= 10;
                                sortable_score += 1;
                                filterable_score += 1;
                            }
                        }
                    }
                    serde_json::Value::Object(_) => (),
                }
            }
            let mut settings: Vec<Setting> = Vec::new();

            let per_field_prob = per_field_values
                .values()
                .map(|count| *count as f64 / nb_values as f64)
                .collect::<Vec<_>>();
            let entropy = entropy(per_field_prob);

            if avg_inner_field_entropy_total > 0 {
                avg_inner_field_entropy /= avg_inner_field_entropy_total as f64;
            }

            let truc = per_field_values.keys().cloned().collect::<Vec<_>>();
            let truc = truc.join("");
            let truc = entropy_of(truc.chars());

            println!("{entropy:<6}\t for basic field {field}");
            println!("{:<6}\t for inner field {field}", avg_inner_field_entropy);
            println!("{truc:<6}\t for truc field {field}");

            let update_by = total as isize / 20;

            if field.starts_with("id_") || field.ends_with("_id") {
                searchable_score -= update_by * 5;
                filterable_score += update_by;
            }
            if field.starts_with("id") || field.ends_with("id") {
                searchable_score -= update_by;
                filterable_score += update_by;
            }

            if (4.0..5.5).contains(&truc) {
                println!("updating searchable: 1");
                searchable_score += update_by;
                filterable_score -= update_by;
                sortable_score -= update_by;
            } else {
                searchable_score -= update_by;
            }

            if (3.0..4.0).contains(&avg_inner_field_entropy) {
                println!("updating searchable: 2");
                searchable_score += update_by;
                filterable_score -= update_by;
                sortable_score -= update_by;
            } else {
                searchable_score -= update_by;
            }

            if (12.0..20.0).contains(&entropy) {
                println!("updating searchable: 3");
                searchable_score += update_by;
                filterable_score -= update_by;
                sortable_score -= update_by;
            } else {
                searchable_score -= update_by;
            }

            if (..4.85).contains(&truc) {
                println!("updating filterable: 1");
                filterable_score += update_by;
                sortable_score += update_by;
            } else {
                searchable_score -= update_by;
            }

            if (..2.85).contains(&avg_inner_field_entropy) {
                println!("updating filterable: 2");
                filterable_score += update_by;
                sortable_score += update_by;
            } else {
                searchable_score -= update_by;
            }

            if (..13.0).contains(&entropy) {
                println!("updating filterable: 3");
                filterable_score += update_by;
                sortable_score += update_by;
            } else {
                searchable_score -= update_by;
            }

            if dbg!(searchable_score as f64 / total as f64 * 100.) > 80. {
                settings.push(Setting::Searchable);
            }
            if dbg!(filterable_score as f64 / total as f64 * 100.) > 80. {
                settings.push(Setting::Filterable);
            }
            if dbg!(sortable_score as f64 / total as f64 * 100.) > 80. {
                settings.push(Setting::Sortable);
            }

            self.well_defined
                .insert(field.to_string(), Settings(settings));
        }

        self.generate_final_settings()
    }

    fn generate_final_settings(self) -> FinalSettings {
        let mut final_settings = FinalSettings::default();

        for (field, settings) in self.well_defined {
            for setting in settings.0 {
                match setting {
                    Setting::Searchable => final_settings.searchable_attributes.push(field.clone()),
                    Setting::Filterable => {
                        final_settings.filterable_attributes.insert(field.clone());
                    }
                    Setting::Sortable => {
                        final_settings.sortable_attributes.insert(field.clone());
                    }
                }
            }
        }

        final_settings
    }
}

static ONLY_DISPLAY: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // Matching URLs. see https://stackoverflow.com/questions/3809401/what-is-a-good-regular-expression-to-match-a-url
        Regex::new(r"^https?://[a-zA-Z]+\.[a-zA-Z]+(/[^ :]+)*/?$").unwrap(),
        // Matching paths in filesystems. See https://stackoverflow.com/questions/169008/regex-for-parsing-directory-and-filename
        // Regex::new(r"^(/|\\)?(.*)(/|\\)*\.[^/\\]$").unwrap(),
        // Path
        Regex::new(r"^(.*[/\\])([^/\\]*)$").unwrap(),
    ]
});

static FILTER_BUT_NOT_SEARCH: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // uuid v4
        Regex::new(
            r"^[0-9a-fA-F]{8}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{12}$",
        )
        .unwrap(),
    ]
});

static ONLY_SORT_AND_FILTER: Lazy<Vec<Regex>> =
    // Number date and this kind of stuff
    Lazy::new(|| vec![Regex::new(r"^[0-9_\-\+:\.]+$").unwrap()]);

fn look_like_id(s: &str) -> bool {
    let digits = s.chars().filter(char::is_ascii_hexdigit).count();
    let alpha = s.chars().filter(|c| c.is_alphabetic()).count();

    digits > alpha
}

fn entropy_of<Iterator, Element>(elements: Iterator) -> f64
where
    Iterator: IntoIterator<Item = Element>,
    Element: PartialEq + Eq + Hash,
{
    let mut total = 0;
    let mut map: HashMap<Element, usize> = HashMap::new();
    for e in elements {
        total += 1;
        *map.entry(e).or_default() += 1;
    }
    let probas = map
        .into_values()
        .map(|count| count as f64 / total as f64)
        .collect::<Vec<_>>();

    entropy(probas)
}

fn entropy(probas: impl IntoIterator<Item = f64>) -> f64 {
    let mut entropy = 0.;
    for prob in probas {
        if prob > 0. {
            // 'shannon' : 2.,
            // 'natural' : math.exp(1),
            // 'hartley' : 10.
            entropy -= prob * prob.log(2.);
        }
    }

    entropy
}

#[derive(Default, Debug, Serialize)]
pub struct FinalSettings {
    pub searchable_attributes: Vec<String>,
    pub filterable_attributes: BTreeSet<String>,
    pub sortable_attributes: BTreeSet<String>,
}

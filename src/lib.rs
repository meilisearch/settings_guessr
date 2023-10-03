use std::collections::{BTreeSet, HashMap};

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Default, Debug)]
pub struct FieldAccumulator {
    well_defined: HashMap<String, Settings>,
    unknown: HashMap<String, Vec<Value>>,
}

#[derive(Debug, Clone)]
pub struct Settings(Vec<Setting>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Searchable,
    Filterable,
    Sortable,
}

pub type Document = Map<String, Value>;

impl FieldAccumulator {
    pub fn new() -> FieldAccumulator {
        FieldAccumulator::default()
    }

    pub fn push(&mut self, document: &Document) {
        let document = flatten_serde_json::flatten(document);
        for (key, value) in document {
            self.unknown.entry(key).or_default().push(value);
        }
    }

    pub fn finish(mut self) -> FinalSettings {
        for (field, values) in &self.unknown {
            let total = values.len();

            let mut searchable_score: isize = 0;
            let mut filterable_score: isize = 0;
            let mut sortable_score: isize = 0;

            for value in values {
                match value {
                    Value::Null => (),
                    Value::Bool(_) | Value::Number(_) => {
                        filterable_score += 1;
                        sortable_score += 1;
                    }
                    Value::Array(array) => {
                        // TODO: sorting in an array seems like a bad ideas right?
                        sortable_score -= 1;
                        for value in array {
                            match value {
                                Value::Number(_) => filterable_score += 1,
                                Value::String(s) => {
                                    searchable_score += 1;
                                    filterable_score += 1;
                                    for regex in ONLY_DISPLAY.iter() {
                                        if regex.is_match(s) {
                                            searchable_score -= 10;
                                            filterable_score -= 1;
                                        }
                                    }
                                    for regex in FILTER_BUT_NOT_SEARCH.iter() {
                                        if regex.is_match(s) {
                                            filterable_score += 1;
                                            searchable_score -= 10;
                                        }
                                    }
                                }
                                // This has been flattened and should be ignored.
                                Value::Array(_) => (),
                                // TODO: That seems useless right?
                                Value::Bool(_) => (),
                                // null and object can be ignored
                                _ => (),
                            }
                        }
                    }
                    Value::String(s) => {
                        searchable_score += 1;
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
                    Value::Object(_) => (),
                }
            }

            let mut settings: Vec<Setting> = Vec::new();

            if searchable_score as f64 / total as f64 * 100. > 80. {
                settings.push(Setting::Searchable);
            }
            if filterable_score as f64 / total as f64 * 100. > 80. {
                settings.push(Setting::Filterable);
            }
            if sortable_score as f64 / total as f64 * 100. > 80. {
                settings.push(Setting::Sortable);
            }

            self.well_defined
                .insert(field.to_string(), Settings(settings));
            // do stats on the field
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
        Regex::new(
            r"^[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)$",
        )
        .unwrap(),
        // Matching paths in filesystems. See https://stackoverflow.com/questions/169008/regex-for-parsing-directory-and-filename
        Regex::new(r"^(.*[/\\])([^/\\]*)$").unwrap(),
    ]
});

static FILTER_BUT_NOT_SEARCH: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // Matching UUID-v4. see https://ihateregex.io/expr/uuid/
        Regex::new(
            r"^[0-9a-fA-F]{8}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{12}$",
        )
        .unwrap(),
        // Matching paths in filesystems. See https://stackoverflow.com/questions/169008/regex-for-parsing-directory-and-filename
        Regex::new(r"^(.*[/\\])([^/\\]*)$").unwrap(),
    ]
});

static ONLY_SORT_AND_FILTER: Lazy<Vec<Regex>> = Lazy::new(|| vec![Regex::new(r"^[0-9]$").unwrap()]);

#[derive(Default, Debug, Serialize)]
pub struct FinalSettings {
    pub searchable_attributes: Vec<String>,
    pub filterable_attributes: BTreeSet<String>,
    pub sortable_attributes: BTreeSet<String>,
}

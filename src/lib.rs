use std::collections::{BTreeSet, HashMap};

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
    Displayed,
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

    pub fn finish(self) -> FinalSettings {
        let mut final_settings = FinalSettings::default();

        for (field, settings) in self.well_defined {
            for setting in settings.0 {
                match setting {
                    Setting::Displayed => final_settings.displayed_attributes.push(field.clone()),
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

#[derive(Default, Debug, Serialize)]
pub struct FinalSettings {
    pub displayed_attributes: Vec<String>,
    pub searchable_attributes: Vec<String>,
    pub filterable_attributes: BTreeSet<String>,
    pub sortable_attributes: BTreeSet<String>,
}

pub fn hello() -> FinalSettings {
    FinalSettings::default()
}

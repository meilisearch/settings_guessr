use std::collections::HashMap;

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

#[derive(Debug, Clone)]
pub struct Document(Map<String, Value>);

impl FieldAccumulator {
    pub fn new() -> FieldAccumulator {
        FieldAccumulator::default()
    }

    pub fn push(&mut self, document: Document) {
        flatten_serde_json::flatten(&document);
        for (key, value) in document.0 {
            self.unknown.entry(key).or_default().push(value);
        }
    }
}

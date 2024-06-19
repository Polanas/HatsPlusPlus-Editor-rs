use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum Language {
    English,
    Russian,
}

#[derive(Debug)]
pub struct UiText {
    pub language: Language,
    pub data: HashMap<String, HashMap<String, String>>,
}

impl UiText {
    pub fn new(language: Language, text_path: impl AsRef<Path>) -> Self {
        let json = std::fs::read_to_string(text_path.as_ref()).unwrap();
        let json = serde_json::from_str(&json).unwrap();
        Self {
            language,
            data: json,
        }
    }
    pub fn get(&self, text_name: &str) -> String {
        match self.language {
            Language::English => self.data["en"][text_name].clone(),
            Language::Russian => self.data["ru"][text_name].clone(),
        }
    }
}

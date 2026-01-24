use std::collections::HashMap;
use std::fs;

pub fn load_filetype(path: &str) -> HashMap<String, String> {
    let data = fs::read_to_string(path).expect("Failed to read filetype.json");
    let map: HashMap<String, String> =
        serde_json::from_str(&data).expect("Failed to parse filetype.json");
    map
}

pub fn get_filetype(filename: &str, map: &HashMap<String, String>) -> String {
    let ext = filename.split('.').next_back().unwrap();
    map.get(ext).unwrap_or(&"Unknown".to_string()).to_string()
}


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

pub fn load_keywords(filetype: &str) -> Vec<String> {
    let data = fs::read_to_string(&format!("src/{}.json", filetype)).expect("Failed to read syntax json file");
    let map: HashMap<String, Vec<String>> = serde_json::from_str(&data).expect("Failed to parse syntax json file");
    map.get("keywords").unwrap_or(&vec![]).to_vec()
}

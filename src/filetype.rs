use std::collections::HashMap;

static FILETYPE_JSON: &str = include_str!("filetype.json");

pub fn load_filetype() -> HashMap<String, String> {
    serde_json::from_str(FILETYPE_JSON).expect("Failed to parse filetype.json")
}

pub fn get_filetype(filename: &str, map: &HashMap<String, String>) -> String {
    let ext = filename.split('.').next_back().unwrap_or("");
    map.get(ext).unwrap_or(&"Unknown".to_string()).to_string()
}

fn embed_keywords(filetype: &str) -> &'static str {
    match filetype {
        "c" => include_str!("c.json"),
        "c++" => include_str!("cpp.json"),
        "css" => include_str!("css.json"),
        "go" => include_str!("go.json"),
        "html" => include_str!("html.json"),
        "java" => include_str!("java.json"),
        "javascript" => include_str!("javascript.json"),
        "php" => include_str!("php.json"),
        "python" => include_str!("python.json"),
        "ruby" => include_str!("ruby.json"),
        "rust" => include_str!("rust.json"),
        "typescript" => include_str!("typescript.json"),
        _ => "",
    }
}

pub fn load_keywords(filetype: &str) -> Vec<String> {
    let data = embed_keywords(filetype);
    if data.is_empty() {
        return Vec::new();
    }
    let map: HashMap<String, Vec<String>> = match serde_json::from_str(data) {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };
    map.get("keywords").cloned().unwrap_or_default()
}

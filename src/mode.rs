pub enum Mode {
    Normal,
    Insert,
    Select,
    LineSelect,
    Multiple,
    Evil,
}

pub fn stringify_mode(mode: &Mode) -> String {
    match mode {
        Mode::Select | Mode::LineSelect => "SELECT MODE".to_string(),
        Mode::Normal => "NORMAL MODE".to_string(),
        Mode::Insert => "INSERT MODE".to_string(),
        Mode::Evil => "EVIL MODE".to_string(),
        Mode::Multiple => "MULTI".to_string(),
    }
}

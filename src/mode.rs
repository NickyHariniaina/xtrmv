pub enum Mode {
    Normal,
    Insert,
    Select,
}

pub fn stringify_mode(mode: &Mode) -> String {
    match mode {
        Mode::Select => "SELECT MODE".to_string(),
        Mode::Normal => "NORMAL MODE".to_string(),
        Mode::Insert => "INSERT MODE".to_string(),
    }
}

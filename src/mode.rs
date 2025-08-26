pub enum Mode {
    Normal,
    Insert,
    Evil,
    Xtrm,
}

pub fn stringify_mode(mode: &Mode) -> String {
    match mode {
        Mode::Normal => "NORMAL MODE".to_string(),
        Mode::Insert => "INSERT MODE".to_string(),
        Mode::Evil => "EVIL MODE".to_string(),
        Mode::Xtrm => "XTRM MODE".to_string(),
    }
}

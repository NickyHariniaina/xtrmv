mod utils;
use std::{env, io::Result};

mod editor;
mod key;
mod mode;
mod raw;
mod row;
use crate::editor::Editor;

fn main() -> Result<()> {
    let mut editor = Editor::new()?;
    if let Some(filename) = env::args().nth(1) {
        editor.open(&filename)?;
    }

    editor.set_status_message("HELP: Ctrl-S = save | Ctrl-Q = quit | Ctrl-F = find");

    editor.refresh_screen();
    while editor.try_starting_process() {
        editor.refresh_screen();
    }
    Ok(())
}

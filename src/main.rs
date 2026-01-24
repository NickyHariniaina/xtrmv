mod utils;
use std::{env, io::Result};

mod editor;
mod key;
mod mode;
mod raw;
mod row;
mod filetype;
use crate::editor::Editor;

fn main() -> Result<()> {
    let mut editor = Editor::new()?;
    if let Some(filename) = env::args().nth(1) {
        editor.open(&filename)?;
    }

    editor.refresh_screen();
    while editor.try_starting_process() {
        editor.refresh_screen();
    }
    Ok(())
}

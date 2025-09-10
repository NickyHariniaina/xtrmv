use std::{
    borrow::Cow,
    cmp,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Error, Result, Stdin, Stdout, Write, stdin, stdout},
    time::{Duration, Instant},
};

#[macro_export]
macro_rules! ctrl_key {
    ($k:expr) => {
        $k & 0x1f
    };
}

pub const CTRL_C: u8 = ctrl_key!(b'c');
pub const CTRL_H: u8 = ctrl_key!(b'h');
pub const BACKSPACE: u8 = 127;

use libc::{STDOUT_FILENO, TIOCGWINSZ, winsize};

use crate::{
    RawMode, Row, byte_slice, editor_read_key,
    key::Key,
    mode::{Mode, stringify_mode},
};

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct Editor {
    pub _mode: RawMode,
    pub type_mode: Mode,
    pub c_inline_pos: usize,
    pub c_block_pos: usize,
    pub c_inline_pos_with_tab: usize,
    pub start_key: u8,
    pub rowoff: usize,
    pub coloff: usize,
    pub active_rows: usize,
    pub active_cols: usize,
    pub rows: Vec<Row>,
    pub modified: bool,
    pub stdin: Stdin,
    pub stdout: Stdout,
    pub filename: Option<String>,
    pub notification: String,
    pub notification_timeout: Instant,
}

impl Editor {
    pub fn new() -> Result<Self> {
        let mode = RawMode::enable_raw_mode()?;
        let (rows, cols) = get_window_size()?;
        let stdin = stdin();
        let stdout = stdout();
        Ok(Self {
            _mode: mode,
            type_mode: Mode::Normal,
            c_inline_pos: 0,
            c_block_pos: 0,
            c_inline_pos_with_tab: 0,
            start_key: 0,
            rowoff: 0,
            coloff: 0,
            active_rows: (rows - 2) as usize,
            active_cols: cols as usize,
            rows: Vec::new(),
            modified: false,
            stdin,
            stdout,
            filename: None,
            notification: String::new(),
            notification_timeout: Instant::now(),
        })
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<()> {
        self.stdout.write_all(buf)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.stdout.flush()
    }

    pub fn scroll(&mut self) {
        self.c_inline_pos_with_tab = self.c_inline_pos;
        if self.c_block_pos < self.numrows() {
            self.c_inline_pos_with_tab = self.rows[self.c_block_pos].cx_to_rx(self.c_inline_pos);
        }

        if self.c_block_pos < self.rowoff {
            self.rowoff = self.c_block_pos;
        }

        if self.c_block_pos >= self.rowoff + self.active_rows {
            self.rowoff = self.c_block_pos - self.active_rows + 1;
        }
        if self.c_inline_pos_with_tab < self.coloff {
            self.coloff = self.c_inline_pos_with_tab;
        }
        if self.c_inline_pos_with_tab >= self.coloff + self.active_cols {
            self.coloff = (1 + self.c_inline_pos_with_tab) - self.active_cols;
        }
    }

    pub fn numrows(&self) -> usize {
        self.rows.len()
    }

    pub fn draw_rows(&mut self) -> Result<()> {
        let numrows = self.numrows();
        let mut count = 0;

        for y in 0..(self.active_rows) {
            let filerow = y + self.rowoff;
            if filerow >= numrows {
                if self.filename.is_none()
                    && self.rows.is_empty()
                    && (y >= self.active_rows / 3 && count != 5)
                {
                    let mut msg = "XTRMV".to_string();
                    if count == 1 {
                        msg = "Have fun with our CLI text editor.".to_string();
                    } else if count == 2 {
                        msg = "You can explore various Mode by using the keymaps".to_string();
                    } else if count == 4 {
                        msg = "You can read the documentation by pressing :help or quit with :q!"
                            .to_string();
                    } else if count == 3 {
                        msg = "-----------------".to_string();
                    }
                    count += 1;
                    msg.truncate(self.active_cols);
                    let padding = (self.active_cols - msg.len()) / 2;
                    if padding > 0 {
                        self.write(b"-")?;
                        for _ in 1..padding {
                            self.write(b" ")?;
                        }
                    }
                    self.write(msg.as_bytes())?;
                } else {
                    self.write(b"-")?;
                }
            } else {
                self.stdout.write_all(byte_slice(
                    &self.rows[filerow].render,
                    self.coloff,
                    self.active_cols,
                ))?;
            }

            self.write(b"\x1b[K")?;
            self.write(b"\r\n")?;
        }
        Ok(())
    }

    pub fn draw_status_bar(&mut self) -> Result<()> {
        self.write(b"\x1b[7m")?; // revert background color
        let status;
        {
            let mode = stringify_mode(&self.type_mode);
            let name = self.filename.as_ref().map_or("[No name]", |s| s.as_str());
            let modified = if self.modified { " (modified)" } else { "" };
            let mut content = format!(
                "-- {} ------ {:.20} - {} lines{}",
                mode,
                name,
                self.numrows(),
                modified
            );
            // :.20 print only 20 letter
            content.truncate(self.active_cols);
            status = content;
        }
        let visual_c_pos_inline = self.c_inline_pos_with_tab + 1;
        let visual_c_pos_block = self.c_block_pos + 1;
        let right_status_content = format!("{}:{}", visual_c_pos_inline, visual_c_pos_block);

        self.write(status.as_bytes())?;
        let mut len = status.len();
        while len < self.active_cols {
            // This part is trying to put the right status content when the active cols - len of
            // the remain status space is equals to the right status
            if self.active_cols - len == right_status_content.len() {
                self.write(right_status_content.as_bytes())?;
                break;
            } else {
                self.write(b" ")?;
                len += 1;
            }
        }
        self.write(b"\x1b[m")?; // RESET
        self.write(b"\r\n")
    }

    pub fn draw_message_bar(&mut self) -> Result<()> {
        self.write(b"\x1b[K")?;
        if !self.notification.is_empty()
            && self.notification_timeout.elapsed() < Duration::from_secs(5)
        {
            let mut msg = Cow::from(self.notification.as_str());
            if msg.len() > self.active_cols {
                msg.to_mut().truncate(self.active_cols);
            }
            self.stdout.write_all(msg.as_bytes())?;
        }
        Ok(())
    }

    pub fn move_cursor_x_times(
        &mut self,
        mut row_times: usize,
        mut col_times: usize,
        direction: Direction,
    ) -> Result<()> {
        match direction {
            Direction::Up => {
                while row_times > 0 {
                    self.move_cursor_up();
                    row_times -= 1;
                }
            }
            Direction::Down => {
                while row_times > 0 {
                    self.move_cursor_down();
                    row_times -= 1;
                }
            }
            Direction::Left => {
                while col_times > 0 {
                    self.move_cursor_left();
                    col_times -= 1;
                }
            }
            Direction::Right => {
                while col_times > 0 {
                    self.move_cursor_right();
                    col_times -= 1;
                }
            }
        }
        Ok(())
    }

    pub fn try_refresh_screen(&mut self) -> Result<()> {
        self.scroll();

        self.write(b"\x1b[?25l")?; // Hide the cursor.
        self.write(b"\x1b[H")?; // Replace cursor at 1,1.

        self.draw_rows()?;
        self.draw_status_bar()?;
        self.draw_message_bar()?;

        let move_cursor = format!(
            "\x1b[{};{}H",
            (self.c_block_pos - self.rowoff) + 1,
            (self.c_inline_pos_with_tab - self.coloff) + 1
        )
        .into_bytes();
        // c_block_pos - self.rowoff is the cursor position relative to what's visible.
        // it is dynamic
        self.write(&move_cursor)?;

        self.write(b"\x1b[?25h")?; // Show the real terminal cursor
        self.flush()
    }

    pub fn refresh_screen(&mut self) {
        self.try_refresh_screen().expect("Failed to refresh screen");
    }

    pub fn set_status_message<S: Into<String>>(&mut self, msg: S) {
        self.notification = msg.into();
        self.notification_timeout = Instant::now();
    }

    pub fn rowlen(&self, index: usize) -> usize {
        let row = self.rows.get(index);
        row.map_or(0, |r| r.characters.len())
    }

    pub fn prompt<F, C>(&mut self, format_prompt: F, mut callback: C) -> Option<String>
    where
        F: Fn(&str) -> String,
        C: FnMut(&mut Self, &str, Key),
    {
        let mut buf = String::new();
        loop {
            self.set_status_message(format_prompt(&buf));
            self.refresh_screen();

            let k = editor_read_key(&mut self.stdin);
            match k {
                Key::Delete | Key::Character(CTRL_H) | Key::Character(BACKSPACE) => {
                    buf.pop();
                }
                Key::Character(b'\x1b') => {
                    self.set_status_message("");
                    callback(self, &buf, k);
                    return None;
                }
                Key::Character(b'\r') => {
                    if !buf.is_empty() {
                        self.set_status_message("");
                        callback(self, &buf, k);
                        return Some(buf);
                    }
                }
                Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown => {
                    callback(self, &buf, k);
                }
                Key::Character(c) if (32..127).contains(&c) => {
                    buf.push(c as char);
                    callback(self, &buf, k);
                }
                Key::Character(CTRL_C) => {
                    self.type_mode = Mode::Normal;
                }
                _ => (),
            }
        }
    }

    pub fn move_cursor_with_vim_key(&mut self, k: Key) -> bool {
        match k {
            Key::Character(b'k') => {
                if self.c_block_pos > 0 {
                    self.c_block_pos -= 1;
                }
            }
            Key::Character(b'j') => {
                if self.numrows() != 0 && self.c_block_pos < self.numrows() - 1 {
                    self.c_block_pos += 1;
                }
            }
            Key::Character(b'h') => {
                if self.c_inline_pos > 0 {
                    self.c_inline_pos -= 1;
                } else if self.c_block_pos > 0 {
                    self.c_block_pos -= 1;
                    self.c_inline_pos = self.rowlen(self.c_block_pos);
                }
            }
            Key::Character(b'l') => {
                let row = self.rows.get(self.c_block_pos);
                let rowlen = row.map_or(0, |r| r.characters.len());
                if self.c_inline_pos < rowlen {
                    self.c_inline_pos += 1;
                } else if row.is_some() && self.c_inline_pos == rowlen {
                    self.c_inline_pos = 0;
                    self.c_block_pos += 1;
                }
            }
            _ => (),
        }
        true
    }

    pub fn move_cursor_left(&mut self) {
        if self.c_inline_pos > 0 {
            self.c_inline_pos -= 1;
        } else if self.c_block_pos > 0 {
            self.c_block_pos -= 1;
            self.c_inline_pos = self.rowlen(self.c_block_pos);
        }
    }

    pub fn move_cursor_right(&mut self) {
        let row = self.rows.get(self.c_block_pos);
        let rowlen = row.map_or(0, |r| r.characters.len());
        if self.c_inline_pos < rowlen {
            self.c_inline_pos += 1;
        } else if row.is_some() && self.c_inline_pos == rowlen {
            self.c_inline_pos = 0;
            self.c_block_pos += 1;
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.numrows() != 0 && self.c_block_pos < self.numrows() - 1 {
            self.c_block_pos += 1;
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.c_block_pos > 0 {
            self.c_block_pos -= 1;
        }
    }

    pub fn move_cursor_with_arrow_key(&mut self, k: Key) {
        match k {
            Key::ArrowUp => {
                self.move_cursor_up();
            }
            Key::ArrowDown => {
                self.move_cursor_down();
            }
            Key::ArrowLeft => {
                self.move_cursor_left();
            }
            Key::ArrowRight => {
                self.move_cursor_right();
            }
            _ => (),
        }

        let rowlen = self.rowlen(self.c_block_pos);
        if self.c_inline_pos > rowlen {
            self.c_inline_pos = rowlen;
        }
    }

    pub fn insert_char(&mut self, c: char) {
        if self.c_block_pos == self.rows.len() {
            self.rows.push(Row::new(""));
        }
        self.rows[self.c_block_pos].insert_char(self.c_inline_pos, c);
        self.c_inline_pos += 1;
        self.modified = true;
    }

    pub fn insert_new_line(&mut self) {
        if self.c_inline_pos == 0 {
            self.rows.insert(self.c_block_pos, Row::new(""));
        } else {
            let new_line = self.rows[self.c_block_pos].truncate(self.c_inline_pos);
            self.rows.insert(self.c_block_pos + 1, Row::new(new_line));
        }
        self.c_block_pos += 1;
        self.c_inline_pos = 0;
    }

    pub fn delete_char(&mut self) {
        if self.c_block_pos == self.rows.len() {
            return;
        }
        if self.c_block_pos == 0 && self.c_inline_pos == 0 {
            return;
        }
        if self.c_inline_pos > 0 {
            self.c_inline_pos -= 1;
            self.rows[self.c_block_pos].delete_char(self.c_inline_pos);
        } else {
            let right = self.rows.remove(self.c_block_pos);
            self.c_block_pos -= 1;
            let left = &mut self.rows[self.c_block_pos];
            self.c_inline_pos = left.characters.len();
            left.append_str(right.characters.as_str());
        }
        self.modified = true;
    }

    pub fn try_starting_process(&mut self) -> bool {
        match self.type_mode {
            Mode::Normal => self.process_keypress(Mode::Normal),
            Mode::Insert => self.process_keypress(Mode::Insert),
            Mode::Multiple => self.process_keypress(Mode::Multiple),
            Mode::Evil => self.process_keypress(Mode::Evil),
        }
    }

    pub fn process_keypress(&mut self, current_mode: Mode) -> bool {
        let c = editor_read_key(&mut self.stdin);
        match current_mode {
            Mode::Insert => self.insert_process(c),
            Mode::Normal => self.normal_process(c),
            Mode::Multiple => true,
            Mode::Evil => true,
        }
    }

    pub fn cmd_process(&mut self) -> bool {
        let command = self.prompt(|v| format!(":{}", v), |_, _, _| ());
        if let Some(cmd) = command {
            if cmd == "q!" {
                return false;
            } else if cmd == "q" {
                if self.modified {
                    let msg = "WARNING!!! File has unsaved changes. \
                         Press :q! to quit without saving."
                        .to_string();

                    self.set_status_message(msg);
                    return true;
                } else {
                    return false;
                }
            } else if cmd == "w" {
                self.save();
            } else if cmd == "wq" || cmd == "x" {
                self.save();
                return false;
            }
        }
        true
    }

    pub fn normal_process(&mut self, c: Key) -> bool {
        match c {
            Key::Character(b'k')
            | Key::Character(b'j')
            | Key::Character(b'l')
            | Key::Character(b'h') => self.move_cursor_with_vim_key(c),
            Key::Character(b'i') => {
                self.type_mode = Mode::Insert;
                true
            }
            Key::Character(b'a') => {
                self.type_mode = Mode::Insert;
                self.move_cursor_x_times(0, 1, Direction::Right).unwrap();
                true
            }
            Key::Character(b'o') => {
                self.type_mode = Mode::Insert;
                self.c_inline_pos = self.rowlen(self.c_block_pos);
                self.insert_new_line();
                true
            }
            Key::Character(b'O') => {
                self.type_mode = Mode::Insert;
                self.c_inline_pos = 0;
                self.insert_new_line();
                self.c_block_pos -= 1;
                true
            }
            Key::Character(b'H') => {
                self.c_block_pos = 0;
                self.c_inline_pos = 0;
                true
            }
            Key::Character(b'L') => {
                self.c_inline_pos = 0;
                self.c_block_pos = self.rows.len() - 1;
                true
            }
            Key::Character(b'0') => {
                self.c_inline_pos = 0;
                true
            }
            Key::Character(b'$') => {
                self.c_inline_pos = self.rowlen(self.c_block_pos);
                true
            }
            Key::Character(b'M') => {
                let middle_pos = (self.rows.len() - 1) / 2;
                self.c_block_pos = middle_pos;
                true
            }
            Key::Character(b'G') => {
                self.c_block_pos = self.rows.len() - 1;
                self.c_inline_pos = 0;
                true
            }
            Key::Character(b'g') => {
                self.start_key = b'g';
                self.double_key_press(self.start_key)
            }
            Key::Character(b':') => self.cmd_process(),
            Key::Character(b'w') | Key::Character(b'b') | Key::Character(b'e') => {
                self.move_by_space(c)
            }
            Key::Character(bkey) => {
                if bkey.is_ascii_digit() {
                    let (repetion_count, last_pressed_key) = self.multiple_key_press(bkey);
                    if last_pressed_key == b'j' {
                        self.move_cursor_x_times(repetion_count, 0, Direction::Down)
                            .unwrap();
                    } else if last_pressed_key == b'k' {
                        self.move_cursor_x_times(repetion_count, 0, Direction::Up)
                            .unwrap();
                    } else if last_pressed_key == b'l' {
                        self.move_cursor_x_times(0, repetion_count, Direction::Right)
                            .unwrap();
                    } else if last_pressed_key == b'h' {
                        self.move_cursor_x_times(0, repetion_count, Direction::Left)
                            .unwrap();
                    }
                }
                true
            }
            _ => true,
        }
    }

    pub fn multiple_key_press(&mut self, first_key: u8) -> (usize, u8) {
        let mut keys_string = String::from((first_key - 48).to_string().as_str());
        let mut number: usize = 0;
        let mut key = editor_read_key(&mut self.stdin);
        let mut last_key_press: u8 = 0;
        while let Key::Character(x) = key {
            if !x.is_ascii_digit() {
                number = match keys_string.parse::<usize>() {
                    Ok(n) => n,
                    Err(_e) => {
                        return (0, 0);
                    }
                };
                last_key_press = x;
                break;
            } else {
                let current_byte_to_number = x - 48;
                keys_string.push_str(current_byte_to_number.to_string().as_str());
            }
            key = editor_read_key(&mut self.stdin);
        }
        (number, last_key_press)
    }

    pub fn double_key_press(&mut self, first_byte_key: u8) -> bool {
        let second_key = editor_read_key(&mut self.stdin);
        if first_byte_key == b'g' && second_key == Key::Character(b'g') {
            self.c_block_pos = 0;
            self.c_inline_pos = 0;
        }
        true
    }

    pub fn move_by_space(&mut self, k: Key) -> bool {
        if let Some(r) = self.rows.get(self.c_block_pos) {
            let chars: Vec<char> = r.characters.chars().collect();
            loop {
                match k {
                    Key::Character(b'w') => {
                        if self.c_inline_pos >= chars.len() {
                            if self.c_block_pos + 1 < self.rows.len() {
                                self.c_block_pos += 1;
                                self.c_inline_pos = 0;
                                return true;
                            } else {
                                break;
                            }
                        }

                        if chars[self.c_inline_pos] == ' '
                            && (self.c_inline_pos + 1 < chars.len()
                                && chars[self.c_inline_pos + 1] != ' ')
                        {
                            self.c_inline_pos += 1;
                            break;
                        } else if self.c_inline_pos >= chars.len() - 1 {
                            self.c_inline_pos = chars.len();
                            break;
                        } else {
                            self.c_inline_pos += 1;
                        }
                    }

                    Key::Character(b'b') => {
                        if self.c_inline_pos == 0 {
                            if self.c_block_pos == 0 {
                                break;
                            } else {
                                self.c_block_pos -= 1;
                                self.c_inline_pos = self.rowlen(self.c_block_pos);
                                return true;
                            }
                        }

                        if self.c_inline_pos >= chars.len() {
                            self.c_inline_pos = chars.len() - 1;
                        }

                        if chars[self.c_inline_pos] != ' '
                            && self.c_inline_pos > 0
                            && chars[self.c_inline_pos - 1] == ' '
                        {
                            self.c_inline_pos -= 1;
                            break;
                        } else {
                            self.c_inline_pos -= 1;
                        }
                    }
                    _ => return true,
                }
            }
        }
        true
    }

    pub fn insert_tab(&mut self) {
        for _i in 0..2 {
            self.insert_char(' ');
            self.write(b" ").unwrap();
        }
    }

    pub fn insert_process(&mut self, c: Key) -> bool {
        match c {
            Key::Character(CTRL_C) => {
                self.type_mode = Mode::Normal;
            }
            Key::Character(b'\t') => self.insert_tab(),
            Key::Character(b'\r') => self.insert_new_line(),
            Key::Character(CTRL_H) | Key::Character(BACKSPACE) => self.delete_char(),
            Key::Delete => {
                self.move_cursor_with_arrow_key(Key::ArrowRight);
                self.delete_char();
            }
            Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight => {
                self.move_cursor_with_arrow_key(c);
            }
            Key::Character(k) if (32..127).contains(&k) => self.insert_char(k as char),
            _ => (),
        };
        true
    }

    pub fn open(&mut self, filename: &str) -> Result<()> {
        self.filename = Some(filename.to_owned());
        let f = File::open(filename)?;
        let file = BufReader::new(&f);
        let results: Result<Vec<Row>> = file.lines().map(|r| r.map(Row::new)).collect();
        self.rows = results?;
        self.modified = false;
        Ok(())
    }

    pub fn save_to_file(&mut self) -> Result<usize> {
        let filename = match self.filename {
            Some(ref f) => f,
            None => return Ok(0),
        };
        let mut data: Vec<u8> = Vec::new();
        for row in &self.rows {
            writeln!(data, "{}", &row.characters)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)?;
        file.set_len(data.len() as u64)?;
        file.write_all(&data)?;
        self.modified = false;
        Ok(data.len())
    }

    pub fn save(&mut self) {
        if self.filename.is_none() {
            self.filename = self.prompt(|v| format!("Save as: {}", v), |_, _, _| ());
            if self.filename.is_none() {
                self.set_status_message("Save aborted");
                return;
            }
        }
        match self.save_to_file() {
            Ok(size) => self.set_status_message(format!("{} bytes written to disk", size)),
            Err(e) => self.set_status_message(format!("Can't save! I/O error: {}", e)),
        }
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        clear_screen(&mut self.stdout).expect("Failed to clear screen");
    }
}

pub fn clear_screen(stdout: &mut Stdout) -> Result<()> {
    stdout.write_all(b"\x1b[2J")?;
    stdout.write_all(b"\x1b[H")?;
    stdout.flush()
}

pub fn get_window_size() -> Result<(u16, u16)> {
    let ws = winsize {
        ws_col: 0,
        ws_row: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        if libc::ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws) == -1 || ws.ws_col == 0 {
            return Err(Error::other("get_window_size: ioctl failed"));
        }
    }
    Ok((ws.ws_row, ws.ws_col))
}

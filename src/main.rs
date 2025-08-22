mod editor;
use libc::{STDIN_FILENO, STDOUT_FILENO, TIOCGWINSZ, winsize};
use std::{
    env,
    io::{Error, ErrorKind, Read, Result, Stdin, Stdout, Write},
};

use termios::{
    BRKINT, CS8, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP, IXON, OPOST, TCSAFLUSH, Termios,
    VMIN, VTIME, tcsetattr,
};

use crate::editor::Editor;

const TAB_STOP: usize = 8;

const KILO_QUIT_TIMES: u8 = 3;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Key {
    Character(u8),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
}

struct RawMode {
    origin_terminal: Termios,
}

impl RawMode {
    fn enable_raw_mode() -> Result<Self> {
        let mut terminal = Termios::from_fd(STDIN_FILENO)?;
        let mode = Self {
            origin_terminal: terminal,
        };

        terminal.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        terminal.c_oflag &= !OPOST;
        terminal.c_cflag |= CS8;
        terminal.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        terminal.c_cc[VMIN] = 0;
        terminal.c_cc[VTIME] = 1;

        tcsetattr(STDIN_FILENO, TCSAFLUSH, &terminal)?;
        Ok(mode)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        tcsetattr(STDIN_FILENO, TCSAFLUSH, &self.origin_terminal)
            .expect("Failed to drop raw mode. OUPS");
    }
}

struct Row {
    characters: String,
    render: String,
}

impl Row {
    fn render_row(chars: &str) -> String {
        let mut idx = 0;
        let mut render = String::new();
        for ch in chars.chars() {
            if ch == '\t' {
                render.push(' ');
                idx += 1;
                while idx % TAB_STOP != 0 {
                    render.push(' ');
                    idx += 1;
                }
            } else {
                render.push(ch);
                idx += 1;
            }
        }
        render
    }

    fn new<T>(s: T) -> Self
    where
        T: Into<String>,
    {
        let chars = s.into();
        let render = Self::render_row(&chars);
        Self {
            characters: chars,
            render,
        }
    }

    fn cx_to_rx(&self, cx: usize) -> usize {
        let mut rx = 0;
        for ch in self.characters.chars().take(cx) {
            if ch == '\t' {
                rx += (TAB_STOP - 1) - (rx % TAB_STOP);
            }
            rx += 1;
        }
        rx
    }

    fn insert_char(&mut self, at: usize, c: char) {
        let idx = if at > self.characters.len() {
            self.characters.len()
        } else {
            at
        };
        self.characters.insert(idx, c);
        self.render = Self::render_row(&self.characters);
    }

    fn delete_char(&mut self, at: usize) {
        if at >= self.characters.len() {
            return;
        }
        self.characters.remove(at);
        self.render = Self::render_row(&self.characters);
    }

    fn append_str(&mut self, s: &str) {
        self.characters.push_str(s);
        self.render = Self::render_row(&self.characters);
    }

    fn truncate(&mut self, new_len: usize) -> String {
        let removed = self.characters[new_len..].to_string();
        self.characters.truncate(new_len);
        self.render = Self::render_row(&self.characters);
        removed
    }
}

fn read_non_blocking<R: Read>(r: &mut R, buf: &mut [u8]) -> usize {
    r.read(buf)
        .or_else(|e| {
            if e.kind() == ErrorKind::WouldBlock {
                Ok(0)
            } else {
                Err(e)
            }
        })
        .expect("Read_non_blocking")
}

fn byte_slice(s: &str, offset: usize, max_len: usize) -> &[u8] {
    if s.len() > max_len + offset {
        &s.as_bytes()[offset..(max_len + offset)]
    } else if s.len() > offset {
        &s.as_bytes()[offset..]
    } else {
        &s.as_bytes()[0..0]
    }
}

fn read_escape_sequence(stdin: &mut Stdin) -> Key {
    let mut seq = [0; 2];
    let n = read_non_blocking(stdin, &mut seq);
    if n == 2 && seq[0] == b'[' {
        if seq[1] >= b'0' && seq[1] <= b'9' {
            let mut last = [0; 1];
            if read_non_blocking(stdin, &mut last) == 1 && last[0] == b'~' {
                match seq[1] {
                    b'1' | b'7' => Key::Home,
                    b'3' => Key::Delete,
                    b'4' | b'8' => Key::End,
                    b'5' => Key::PageUp,
                    b'6' => Key::PageDown,
                    _ => Key::Character(b'\x1b'),
                }
            } else {
                Key::Character(b'\x1b')
            }
        } else {
            match seq[1] {
                b'A' => Key::ArrowUp,
                b'B' => Key::ArrowDown,
                b'C' => Key::ArrowRight,
                b'D' => Key::ArrowLeft,
                b'H' => Key::Home,
                b'F' => Key::End,
                _ => Key::Character(b'\x1b'),
            }
        }
    } else if n == 2 && seq[0] == b'O' {
        match seq[1] {
            b'H' => Key::Home,
            b'F' => Key::End,
            _ => Key::Character(b'\x1b'),
        }
    } else {
        Key::Character(b'\x1b')
    }
}

fn editor_read_key(stdin: &mut Stdin) -> Key {
    let mut buf = [0; 1];
    loop {
        if read_non_blocking(stdin, &mut buf) == 1 {
            return match buf[0] {
                b'\x1b' => read_escape_sequence(stdin),
                ch => Key::Character(ch),
            };
        }
    }
}
fn get_window_size() -> Result<(u16, u16)> {
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

fn clear_screen(stdout: &mut Stdout) -> Result<()> {
    stdout.write_all(b"\x1b[2J")?;
    stdout.write_all(b"\x1b[H")?;
    stdout.flush()
}

fn main() -> Result<()> {
    let mut editor = Editor::new()?;
    if let Some(filename) = env::args().nth(1) {
        editor.open(&filename)?;
    }

    editor.set_status_message("HELP: Ctrl-S = save | Ctrl-Q = quit | Ctrl-F = find");

    editor.refresh_screen();
    while editor.process_keypress() {
        editor.refresh_screen();
    }
    Ok(())
}

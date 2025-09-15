use libc::STDIN_FILENO;
use termios::{
    BRKINT, CS8, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP, IXON, OPOST, TCSAFLUSH, Termios,
    VMIN, VTIME, tcsetattr,
};

pub struct RawMode {
    pub origin_terminal: Termios,
}
use std::io::Result;
impl RawMode {
    pub fn enable_raw_mode() -> Result<Self> {
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

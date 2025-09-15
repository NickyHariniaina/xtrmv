use crate::utils::TAB_STOP;

pub struct Row {
    pub characters: String,
    pub render: String,
}

impl Row {
    pub fn render_row(chars: &str) -> String {
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

    pub fn new<T>(s: T) -> Self
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

    pub fn cx_to_rx(&self, cx: usize) -> usize {
        let mut rx = 0;
        for ch in self.characters.chars().take(cx) {
            if ch == '\t' {
                rx += (TAB_STOP - 1) - (rx % TAB_STOP);
            }
            rx += 1;
        }
        rx
    }

    pub fn insert_char(&mut self, at: usize, c: char) {
        let idx = if at > self.characters.len() {
            self.characters.len()
        } else {
            at
        };
        self.characters.insert(idx, c);
        self.render = Self::render_row(&self.characters);
    }

    pub fn delete_char(&mut self, at: usize) {
        if at >= self.characters.len() {
            return;
        }
        self.characters.remove(at);
        self.render = Self::render_row(&self.characters);
    }

    pub fn append_str(&mut self, s: &str) {
        self.characters.push_str(s);
        self.render = Self::render_row(&self.characters);
    }

    pub fn truncate(&mut self, new_len: usize) -> String {
        let removed = self.characters[new_len..].to_string();
        self.characters.truncate(new_len);
        self.render = Self::render_row(&self.characters);
        removed
    }
}

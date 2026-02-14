use crate::{filetype::load_keywords, utils::TAB_STOP};

// #[derive(Debug, Clone, Copy)]
// pub enum Highlight {
//     Normal,
//     Number,
//     String,
//     Comment,
//     Keyword,
// }

pub struct Row {
    pub characters: String,
    pub render: String,
}

impl Row {
    pub fn update_render_with_syntax(&mut self, filetype: &str) {
        let keywords = load_keywords(filetype);

        let mut render = String::new();

        let chars: Vec<char> = self.characters.chars().collect();
        let mut i = 0;
        let mut is_inside_comment = false;

        while i < chars.len() {
            let c = chars[i];

            if c == '"' {
                render.push_str("\x1b[32m");
                render.push(c);
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    render.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() {
                    render.push(chars[i]);
                    i += 1;
                }
                render.push_str("\x1b[39m");
                continue;
            }

            if is_inside_comment {
                if c == '\n' {
                    render.push_str("\x1b[0m");
                    is_inside_comment = false;
                }
                render.push(c);
                i += 1;
                continue;
            }

            if (filetype == "ruby" || filetype == "python") && c == '#' {
                render.push_str("\x1b[2;90m");

                while i < chars.len() {
                    render.push(chars[i]);
                    i += 1;
                }

                render.push_str("\x1b[0m");
                break;
            }

            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                render.push_str("\x1b[2;90m");

                while i < chars.len() {
                    render.push(chars[i]);
                    i += 1;
                }

                render.push_str("\x1b[0m");
                break;
            }

            if c.is_ascii_digit() && !is_inside_comment {
                render.push_str("\x1b[35m");
                while i < chars.len() && chars[i].is_ascii_digit() {
                    render.push(chars[i]);
                    i += 1;
                }
                render.push_str("\x1b[39m");
                continue;
            }

            if c.is_ascii_alphabetic() || c == '_' && !is_inside_comment {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                if keywords.contains(&word) {
                    render.push_str("\x1b[34m");
                    render.push_str(&word);
                    render.push_str("\x1b[0m");
                } else if i < chars.len() && chars[i] == '(' {
                    render.push_str("\x1b[1;31m");
                    render.push_str(&word);
                    render.push_str("\x1b[0m");
                } else {
                    render.push_str(&word);
                }
                continue;
            }

            render.push(c);
            i += 1;
        }

        self.render = render;
    }

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

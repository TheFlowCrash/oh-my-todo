use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
pub struct TextInput {
    lines: Vec<String>,
    row: usize,
    col: usize,
    multiline: bool,
}

impl TextInput {
    pub fn new(initial: impl AsRef<str>, multiline: bool) -> Self {
        let mut lines = initial
            .as_ref()
            .split('\n')
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let row = lines.len().saturating_sub(1);
        let col = char_len(&lines[row]);

        Self {
            lines,
            row,
            col,
            multiline,
        }
    }

    pub fn single_line(initial: impl AsRef<str>) -> Self {
        Self::new(initial, false)
    }

    pub fn multiline(initial: impl AsRef<str>) -> Self {
        Self::new(initial, true)
    }

    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_blank(&self) -> bool {
        self.lines.iter().all(|line| line.trim().is_empty())
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.row = row.min(self.lines.len().saturating_sub(1));
        self.col = col.min(char_len(self.current_line()));
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.insert_char(c)
            }
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Up if self.multiline => self.move_up(),
            KeyCode::Down if self.multiline => self.move_down(),
            KeyCode::Home => self.col = 0,
            KeyCode::End => self.col = char_len(self.current_line()),
            KeyCode::Enter if self.multiline => self.insert_newline(),
            _ => {}
        }
    }

    fn current_line(&self) -> &String {
        &self.lines[self.row]
    }

    fn current_line_mut(&mut self) -> &mut String {
        &mut self.lines[self.row]
    }

    fn insert_char(&mut self, c: char) {
        let col = self.col;
        let byte_index = char_to_byte_index(self.current_line(), col);
        self.current_line_mut().insert(byte_index, c);
        self.col += 1;
    }

    fn backspace(&mut self) {
        if self.col > 0 {
            let start = char_to_byte_index(self.current_line(), self.col - 1);
            let end = char_to_byte_index(self.current_line(), self.col);
            self.current_line_mut().replace_range(start..end, "");
            self.col -= 1;
        } else if self.multiline && self.row > 0 {
            let current = self.lines.remove(self.row);
            self.row -= 1;
            self.col = char_len(self.current_line());
            self.current_line_mut().push_str(&current);
        }
    }

    fn delete(&mut self) {
        let line_len = char_len(self.current_line());
        if self.col < line_len {
            let start = char_to_byte_index(self.current_line(), self.col);
            let end = char_to_byte_index(self.current_line(), self.col + 1);
            self.current_line_mut().replace_range(start..end, "");
        } else if self.multiline && self.row + 1 < self.lines.len() {
            let next = self.lines.remove(self.row + 1);
            self.current_line_mut().push_str(&next);
        }
    }

    fn move_left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.multiline && self.row > 0 {
            self.row -= 1;
            self.col = char_len(self.current_line());
        }
    }

    fn move_right(&mut self) {
        let line_len = char_len(self.current_line());
        if self.col < line_len {
            self.col += 1;
        } else if self.multiline && self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
    }

    fn move_up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.col = self.col.min(char_len(self.current_line()));
        }
    }

    fn move_down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = self.col.min(char_len(self.current_line()));
        }
    }

    fn insert_newline(&mut self) {
        let split_index = char_to_byte_index(self.current_line(), self.col);
        let remainder = self.current_line()[split_index..].to_owned();
        self.current_line_mut().truncate(split_index);
        self.row += 1;
        self.lines.insert(self.row, remainder);
        self.col = 0;
    }
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}

fn char_to_byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .map(|(index, _)| index)
        .nth(char_index)
        .unwrap_or(value.len())
}

#[cfg(test)]
mod tests {
    use super::TextInput;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn multiline_backspace_merges_lines() {
        let mut input = TextInput::multiline("hello\nworld");
        input.handle_key(key(KeyCode::Home));
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.value(), "helloworld");
    }

    #[test]
    fn single_line_ignores_enter() {
        let mut input = TextInput::single_line("hello");
        input.handle_key(key(KeyCode::Enter));
        assert_eq!(input.value(), "hello");
    }
}

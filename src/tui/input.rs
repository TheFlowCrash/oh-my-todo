use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Style;
use ratatui::widgets::Block;
use tui_textarea::{CursorMove, TextArea};

#[derive(Debug, Clone)]
pub struct TextInput {
    textarea: TextArea<'static>,
    multiline: bool,
}

impl TextInput {
    pub fn new(initial: impl AsRef<str>, multiline: bool) -> Self {
        let mut textarea = TextArea::from(initial.as_ref().split('\n'));
        textarea.move_cursor(CursorMove::Bottom);
        textarea.move_cursor(CursorMove::End);
        textarea.set_cursor_line_style(Style::default());

        Self {
            textarea,
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
        self.textarea.lines().join("\n")
    }

    pub fn is_blank(&self) -> bool {
        self.textarea
            .lines()
            .iter()
            .all(|line| line.trim().is_empty())
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        let target_row = if self.multiline { row } else { 0 } as u16;
        self.textarea
            .move_cursor(CursorMove::Jump(target_row, col as u16));
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if !self.multiline && matches!(key.code, KeyCode::Enter | KeyCode::Up | KeyCode::Down) {
            return;
        }

        self.textarea.input(key);
    }

    pub fn widget(&self, block: Block<'static>, style: Style) -> TextArea<'static> {
        let mut textarea = self.textarea.clone();
        textarea.set_block(block);
        textarea.set_style(style);
        textarea
    }
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

/// Text input state with cursor management
#[derive(Debug, Default)]
pub struct TextInputState {
    value: String,
    cursor: usize,
}

impl TextInputState {
    pub fn new(value: String) -> Self {
        let cursor = value.len();
        Self { value, cursor }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    /// Get the value with cursor indicator for display
    pub fn display_with_cursor(&self) -> String {
        let mut result = self.value.clone();
        if self.cursor <= result.len() {
            result.insert(self.cursor, '|');
        }
        result
    }
}

/// Input validation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    None,
    Integer,
    Float,
    Identifier, // alphanumeric + underscore
}

impl ValidationMode {
    pub fn validate(&self, c: char) -> bool {
        match self {
            ValidationMode::None => true,
            ValidationMode::Integer => c.is_ascii_digit() || c == '-',
            ValidationMode::Float => c.is_ascii_digit() || c == '-' || c == '.',
            ValidationMode::Identifier => c.is_alphanumeric() || c == '_',
        }
    }

    pub fn validate_string(&self, s: &str) -> bool {
        match self {
            ValidationMode::None => true,
            ValidationMode::Integer => s.parse::<i64>().is_ok(),
            ValidationMode::Float => s.parse::<f64>().is_ok(),
            ValidationMode::Identifier => {
                !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_basic() {
        let mut input = TextInputState::new(String::new());
        input.insert('h');
        input.insert('e');
        input.insert('l');
        input.insert('l');
        input.insert('o');
        assert_eq!(input.value(), "hello");
        assert_eq!(input.cursor(), 5);
    }

    #[test]
    fn test_text_input_backspace() {
        let mut input = TextInputState::new("hello".to_string());
        input.backspace();
        assert_eq!(input.value(), "hell");
    }

    #[test]
    fn test_text_input_cursor_movement() {
        let mut input = TextInputState::new("hello".to_string());
        assert_eq!(input.cursor(), 5);
        input.move_left();
        assert_eq!(input.cursor(), 4);
        input.move_home();
        assert_eq!(input.cursor(), 0);
        input.move_end();
        assert_eq!(input.cursor(), 5);
    }

    #[test]
    fn test_validation_modes() {
        assert!(ValidationMode::Integer.validate('5'));
        assert!(!ValidationMode::Integer.validate('a'));
        assert!(ValidationMode::Float.validate('.'));
        assert!(ValidationMode::Identifier.validate('_'));
        assert!(!ValidationMode::Identifier.validate('-'));
    }
}

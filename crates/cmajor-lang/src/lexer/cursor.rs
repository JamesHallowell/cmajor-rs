use std::str::Chars;

pub(crate) const EOF_CHAR: char = '\0';

pub(crate) struct Cursor<'a> {
    len_consumed: u32,
    chars: Chars<'a>,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            len_consumed: 0,
            chars: input.chars(),
        }
    }

    pub(crate) fn len_consumed(&self) -> u32 {
        self.len_consumed
    }

    pub(crate) fn reset_len_consumed(&mut self) {
        self.len_consumed = 0;
    }

    pub(crate) fn peek(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    pub(crate) fn peek_second(&self) -> char {
        let mut chars = self.chars.clone();
        chars.next();
        chars.next().unwrap_or(EOF_CHAR)
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    pub(crate) fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.len_consumed += c.len_utf8() as u32;
        Some(c)
    }

    pub(crate) fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while predicate(self.peek()) && !self.is_eof() {
            self.bump();
        }
    }
}

use std::str::Chars;

pub(crate) struct Cursor<'a> {
    chars: Chars<'a>,
    bytes_taken: u32,
}

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars(),
            bytes_taken: 0,
        }
    }

    pub fn bytes_taken(&self) -> u32 {
        self.bytes_taken
    }

    pub fn reset_bytes_taken(&mut self) {
        self.bytes_taken = 0;
    }

    pub fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    pub fn peek_two(&self) -> (Option<char>, Option<char>) {
        let mut chars = self.chars.clone();
        (chars.next(), chars.next())
    }

    pub fn peek_three(&self) -> (Option<char>, Option<char>, Option<char>) {
        let mut chars = self.chars.clone();
        (chars.next(), chars.next(), chars.next())
    }

    pub fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.clone().nth(n)
    }

    pub fn exhausted(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    pub fn take(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.bytes_taken += c.len_utf8() as u32;
        Some(c)
    }

    pub fn take_n(&mut self, n: usize) {
        for _ in 0..n {
            if self.take().is_none() {
                break;
            }
        }
    }

    pub fn take_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while !self.exhausted() && predicate(self.peek().unwrap()) {
            self.take();
        }
    }
}

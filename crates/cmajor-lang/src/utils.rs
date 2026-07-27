#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Line(usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Column(usize);

pub fn line_col(source: &str, offset: u32) -> (Line, Column) {
    let offset = (offset as usize).min(source.len());
    let mut line = 1;
    let mut column = 1;
    for ch in source[..offset].chars() {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (Line(line), Column(column))
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

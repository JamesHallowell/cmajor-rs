use {crate::utils::span::Span, std::str::FromStr};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Source<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Line(usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Column(usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SourceLocation {
    pub line: Line,
    pub column: Column,
}

impl<'a> Source<'a> {
    pub fn new(source: &'a str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(
                source
                    .char_indices()
                    .filter_map(|(i, c)| if c == '\n' { Some(i + 1) } else { None }),
            )
            .collect::<Vec<_>>();

        Self {
            source,
            line_starts,
        }
    }

    pub fn as_str(&self) -> &str {
        self.source
    }

    pub fn location(&self, index: u32) -> SourceLocation {
        let index = index as usize;
        let line = self
            .line_starts
            .binary_search(&index)
            .unwrap_or_else(|line| line - 1);
        let column = index - self.line_starts[line];

        SourceLocation {
            line: (line + 1).into(),
            column: (column + 1).into(),
        }
    }

    pub fn index(&self, location: SourceLocation) -> u32 {
        let SourceLocation {
            line: Line(line),
            column: Column(column),
        } = location;

        (self.line_starts[line - 1] + column - 1) as u32
    }
}

impl FromStr for SourceLocation {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (line, col) = s.split_once(':').unwrap_or((s, ""));

        Ok(Self {
            line: line.parse::<usize>().map(Line::from)?,
            column: col.parse::<usize>().map(Column::from)?,
        })
    }
}

impl<'a> From<&'a str> for Source<'a> {
    fn from(source: &'a str) -> Self {
        Self::new(source)
    }
}

impl<'a> From<Source<'a>> for &'a str {
    fn from(source: Source<'a>) -> Self {
        source.source
    }
}

impl<'a> std::ops::Index<Span<u32>> for Source<'a> {
    type Output = str;

    fn index(&self, index: Span<u32>) -> &'a Self::Output {
        &self.source[index.start as usize..index.end as usize]
    }
}

impl<'a> std::ops::Index<Span<SourceLocation>> for Source<'a> {
    type Output = str;

    fn index(&self, span: Span<SourceLocation>) -> &'a Self::Output {
        let span = span.to_position_span(self);
        &self.source[span.start as usize..span.end as usize]
    }
}

impl<'a> std::ops::Index<std::ops::Range<SourceLocation>> for Source<'a> {
    type Output = str;

    fn index(&self, span: std::ops::Range<SourceLocation>) -> &'a Self::Output {
        let span: Span<SourceLocation> = span.into();
        let span = span.to_position_span(self);
        &self.source[span.start as usize..span.end as usize]
    }
}

impl<'a> std::ops::Index<Line> for Source<'a> {
    type Output = str;

    fn index(&self, line: Line) -> &Self::Output {
        let start = SourceLocation {
            line,
            column: 1.into(),
        };
        let end = SourceLocation {
            line: line + 1.into(),
            column: 1.into(),
        };
        let span = Span { start, end };

        &self[span]
    }
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

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl From<Line> for usize {
    fn from(value: Line) -> Self {
        value.0
    }
}

impl From<usize> for Line {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl std::ops::Add for Line {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Line {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl From<Column> for usize {
    fn from(value: Column) -> Self {
        value.0
    }
}

impl From<usize> for Column {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl std::ops::Add for Column {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Column {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl From<(Line, Column)> for SourceLocation {
    fn from((line, column): (Line, Column)) -> Self {
        Self { line, column }
    }
}

#[macro_export]
macro_rules! line {
    ($line:literal) => {
        $crate::utils::source::Line::from($line)
    };
    ($line:ident) => {
        $crate::utils::source::Line::from($line)
    };
}

#[macro_export]
macro_rules! col {
    ($line:literal) => {
        $crate::utils::source::Column::from($line)
    };
    ($line:ident) => {
        $crate::utils::source::Column::from($line)
    };
}

#[macro_export]
macro_rules! location {
    ($line:literal:$column:literal) => {
        $crate::utils::source::SourceLocation {
            line: $crate::line!($line),
            column: $crate::col!($column),
        }
    };
    ($line:literal:$column:ident) => {
        $crate::utils::source::SourceLocation {
            line: $crate::line!($line),
            column: $crate::col!($column),
        }
    };
    ($line:ident:$column:literal) => {
        $crate::utils::source::SourceLocation {
            line: $crate::line!($line),
            column: $crate::col!($column),
        }
    };
    ($line:ident:$column:ident) => {
        $crate::utils::source::SourceLocation {
            line: $crate::line!($line),
            column: $crate::col!($column),
        }
    };
}

#[cfg(test)]
mod tests {
    use {super::*, indoc::indoc};

    const CODE: &str = indoc! {"
        let x = 1;
        let y = 2;
        let z = 3;
    "};

    #[test]
    fn fetching_source_locations() {
        let source: Source = CODE.into();

        assert_eq!(&source[location!(1:1)..location!(1:6)], "let x");
        assert_eq!(&source[location!(2:1)..location!(3:6)], "let y = 2;\nlet z");

        assert_eq!(&source[line!(1)], "let x = 1;\n");
        assert_eq!(&source[line!(2)], "let y = 2;\n");
        assert_eq!(&source[line!(3)], "let z = 3;\n");
    }

    #[test]
    fn mapping_locations_to_index() {
        let source: Source = CODE.into();
        let assert_index = |location: SourceLocation, index: u32| {
            assert_eq!(index, source.index(location));
            assert_eq!(location, source.location(index));
        };

        assert_index(location!(1:1), 0);
        assert_index(location!(1:2), 1);
        assert_index(location!(1:3), 2);
        assert_index(location!(1:4), 3);
        assert_index(location!(1:5), 4);
        assert_index(location!(1:6), 5);
        assert_index(location!(1:7), 6);
        assert_index(location!(1:8), 7);
        assert_index(location!(1:9), 8);
        assert_index(location!(1:10), 9);
        assert_index(location!(1:11), 10);
        assert_index(location!(2:1), 11);
        assert_index(location!(2:2), 12);
        assert_index(location!(2:3), 13);
    }
}

use {crate::utils::span::Span, std::str::FromStr};

#[derive(Debug, Clone)]
pub struct SourceFile {
    name: Option<String>,
    contents: String,
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

impl SourceFile {
    pub fn new(source: impl Into<String>) -> Self {
        let contents = source.into();
        let line_starts = std::iter::once(0)
            .chain(
                contents
                    .char_indices()
                    .filter_map(|(i, c)| if c == '\n' { Some(i + 1) } else { None }),
            )
            .collect::<Vec<_>>();

        Self {
            name: None,
            contents,
            line_starts,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub fn location(&self, index: u32) -> SourceLocation {
        let index = index as usize;
        let line = self
            .line_starts
            .binary_search(&index)
            .unwrap_or_else(|line| line - 1);
        let column = self.contents[self.line_starts[line]..index].chars().count();

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

        let line_start = self.line_starts[line - 1];
        let rest = &self.contents[line_start..];
        let offset = rest
            .char_indices()
            .map(|(offset, _)| offset)
            .chain([rest.len()])
            .nth(column - 1)
            .expect("location is past the end of the source");

        (line_start + offset) as u32
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

impl From<&str> for SourceFile {
    fn from(source: &str) -> Self {
        Self::new(source)
    }
}

impl std::ops::Index<Span<u32>> for SourceFile {
    type Output = str;

    fn index(&self, index: Span<u32>) -> &Self::Output {
        &self.contents[index.start as usize..index.end as usize]
    }
}

impl std::ops::Index<Span<SourceLocation>> for SourceFile {
    type Output = str;

    fn index(&self, span: Span<SourceLocation>) -> &Self::Output {
        let span = span.to_position_span(self);
        &self.contents[span.start as usize..span.end as usize]
    }
}

impl std::ops::Index<std::ops::Range<SourceLocation>> for SourceFile {
    type Output = str;

    fn index(&self, span: std::ops::Range<SourceLocation>) -> &Self::Output {
        let span: Span<SourceLocation> = span.into();
        let span = span.to_position_span(self);
        &self.contents[span.start as usize..span.end as usize]
    }
}

impl std::ops::Index<Line> for SourceFile {
    type Output = str;

    fn index(&self, Line(line): Line) -> &Self::Output {
        let start = self.line_starts[line - 1];
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.contents.len());

        &self.contents[start..end]
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
    fn last_line_without_a_trailing_newline_ends_at_the_end_of_the_source() {
        let source = SourceFile::new("let x = 1;\nlet y = 2;");

        assert_eq!(&source[line!(2)], "let y = 2;");
    }

    #[test]
    fn fetching_source_locations() {
        let source: SourceFile = CODE.into();

        assert_eq!(&source[location!(1:1)..location!(1:6)], "let x");
        assert_eq!(&source[location!(2:1)..location!(3:6)], "let y = 2;\nlet z");

        assert_eq!(&source[line!(1)], "let x = 1;\n");
        assert_eq!(&source[line!(2)], "let y = 2;\n");
        assert_eq!(&source[line!(3)], "let z = 3;\n");
    }

    #[test]
    fn mapping_locations_to_index() {
        let source: SourceFile = CODE.into();
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

    const NON_ASCII: &str = "let é = 1;\nlet 名前 = 2;";

    #[test]
    fn non_ascii_character_moves_the_column_by_one() {
        let source: SourceFile = NON_ASCII.into();
        let equals = NON_ASCII.find('=').unwrap() as u32;

        assert_eq!(source.location(equals), location!(1:7));
    }

    #[test]
    fn column_after_non_ascii_characters_maps_to_its_byte_offset() {
        let source: SourceFile = NON_ASCII.into();
        let second_equals = NON_ASCII.rfind('=').unwrap() as u32;

        assert_eq!(source.index(location!(2:8)), second_equals);
    }

    #[test]
    fn byte_offset_round_trips_through_its_location() {
        let source: SourceFile = NON_ASCII.into();
        let offsets = NON_ASCII
            .char_indices()
            .map(|(offset, _)| offset)
            .chain([NON_ASCII.len()]);

        for offset in offsets {
            let offset = offset as u32;
            assert_eq!(source.index(source.location(offset)), offset);
        }
    }
}

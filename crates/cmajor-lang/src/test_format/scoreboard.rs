use {
    super::{Directive, Outcome, TestResult},
    std::{fmt, iter::Sum, ops::Add},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    passed: usize,
    total: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FileScore {
    pub valid_programs: Tally,
    pub expected_errors: Tally,
}

impl Tally {
    fn record(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Pass => {
                self.passed += 1;
                self.total += 1;
            }
            Outcome::Fail(_) => self.total += 1,
            Outcome::Skipped(_) => {}
        }
    }
}

impl<'a> FromIterator<&'a TestResult> for FileScore {
    fn from_iter<I: IntoIterator<Item = &'a TestResult>>(results: I) -> Self {
        let mut score = FileScore::default();
        for result in results {
            let tally = match result.directive {
                Directive::ExpectError(_) => &mut score.expected_errors,
                _ => &mut score.valid_programs,
            };
            tally.record(&result.outcome);
        }
        score
    }
}

impl Add for Tally {
    type Output = Tally;

    fn add(self, other: Tally) -> Tally {
        Tally {
            passed: self.passed + other.passed,
            total: self.total + other.total,
        }
    }
}

impl Add for FileScore {
    type Output = FileScore;

    fn add(self, other: FileScore) -> FileScore {
        FileScore {
            valid_programs: self.valid_programs + other.valid_programs,
            expected_errors: self.expected_errors + other.expected_errors,
        }
    }
}

impl Sum for FileScore {
    fn sum<I: Iterator<Item = FileScore>>(scores: I) -> Self {
        scores.fold(FileScore::default(), Add::add)
    }
}

impl fmt::Display for Tally {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Tally { passed, total } = *self;
        if total == 0 {
            return f.pad("0/0");
        }
        let percent = 100.0 * passed as f64 / total as f64;
        f.pad(&format!("{passed}/{total} ({percent:.1}%)"))
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            test_format::{ExpectedError, Failure, Skip},
            utils::source::Line,
        },
    };

    fn result(directive: Directive, outcome: Outcome) -> TestResult {
        TestResult {
            directive,
            line: Line::from(1),
            outcome,
        }
    }

    fn expect_error() -> Directive {
        Directive::ExpectError(ExpectedError::default())
    }

    fn tally(passed: usize, total: usize) -> Tally {
        Tally { passed, total }
    }

    #[test]
    fn tally_counts_passes_out_of_checked_results() {
        let results = [
            result(Directive::TestCompile, Outcome::Pass),
            result(Directive::TestProcessor, Outcome::Pass),
            result(
                Directive::TestFunction,
                Outcome::Fail(Failure::Rejected {
                    diagnostics: vec![],
                }),
            ),
        ];

        let score: FileScore = results.iter().collect();

        assert_eq!(score.valid_programs, tally(2, 3));
    }

    #[test]
    fn skipped_results_count_in_neither_tally() {
        let results = [
            result(Directive::TestCompile, Outcome::Skipped(Skip::Disabled)),
            result(expect_error(), Outcome::Skipped(Skip::Disabled)),
            result(
                Directive::Other {
                    name: "runScript".into(),
                },
                Outcome::Skipped(Skip::NotCompiled),
            ),
        ];

        let score: FileScore = results.iter().collect();

        assert_eq!(score, FileScore::default());
    }

    #[test]
    fn expect_error_results_count_as_expected_errors() {
        let results = [
            result(expect_error(), Outcome::Pass),
            result(expect_error(), Outcome::Fail(Failure::NoError)),
        ];

        let score: FileScore = results.iter().collect();

        assert_eq!(
            score,
            FileScore {
                valid_programs: tally(0, 0),
                expected_errors: tally(1, 2),
            }
        );
    }

    #[test]
    fn file_scores_sum_column_by_column() {
        let scores = [
            FileScore {
                valid_programs: tally(1, 2),
                expected_errors: tally(3, 4),
            },
            FileScore {
                valid_programs: tally(5, 6),
                expected_errors: tally(7, 8),
            },
        ];

        assert_eq!(
            scores.into_iter().sum::<FileScore>(),
            FileScore {
                valid_programs: tally(6, 8),
                expected_errors: tally(10, 12),
            }
        );
    }

    #[test]
    fn tally_displays_passes_out_of_total_with_a_percentage() {
        assert_eq!(tally(351, 381).to_string(), "351/381 (92.1%)");
    }

    #[test]
    fn empty_tally_displays_without_a_percentage() {
        assert_eq!(tally(0, 0).to_string(), "0/0");
    }
}

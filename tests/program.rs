use cmajor::{
    diagnostic::{Category, Location, Severity},
    Cmajor,
};

#[test]
fn compile_error() {
    let program = r#"
        processor Test {
            input stweam int in;
            
            fn main() {
                advance();
            }
        }
    "#;

    let cmajor = Cmajor::new_from_env().unwrap();

    let error = match cmajor.parse(program).unwrap_err() {
        cmajor::ParseError::ParserError(parser_error) => parser_error,
        _ => panic!("expected parser error"),
    };

    assert_eq!(error.category(), Some(Category::Compile));
    assert_eq!(error.severity(), Severity::Error);
    assert_eq!(error.message(), "Expected a stream type specifier");
    assert_eq!(error.file_name(), None);
    assert_eq!(
        error.location(),
        Location {
            line: 3,
            column: 19
        }
    );
    assert_eq!(error.source_line(), "            input stweam int in;\n");
    assert_eq!(
        error.annotated_line(),
        "            input stweam int in;\n                  ^"
    );
    assert_eq!(
        error.full_description(),
        "3:19: error: Expected a stream type specifier"
    );
}

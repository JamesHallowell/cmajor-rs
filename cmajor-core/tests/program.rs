use cmajor_core::{
    ast,
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

    let cmajor = Cmajor::new();

    let error = match cmajor.parse(program).unwrap_err() {
        cmajor_core::ParseError::ParserError(parser_error) => parser_error,
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

#[test]
fn get_syntax_tree() {
    let program = r#"
        namespace test
        {
            processor Simple
            {
                output value S out;

                enum E
                {
                    A,
                    B
                }

                struct S
                {
                    bool a;
                    float b;
                    int c;
                    D d;
                    E e;
                }

                struct D
                {
                    int64 a;
                }

                void main()
                {
                    advance();
                }
            }
        }
    "#;

    let cmajor = Cmajor::new();
    let program = cmajor.parse(program).unwrap();
    let syntax_tree = program.get_syntax_tree().unwrap();

    dbg!(&syntax_tree);

    let ast::Node::Namespace(namespace) = syntax_tree else {
        panic!("expected namespace");
    };
    assert_eq!(namespace.name, "_root");

    let ast::Node::Namespace(namespace) = &namespace.sub_modules[0] else {
        panic!("expected namespace");
    };
    assert_eq!(namespace.name, "test");

    let ast::Node::Processor(processor) = &namespace.sub_modules[0] else {
        panic!("expected processor");
    };
    assert_eq!(processor.name, "Simple");

    assert_eq!(processor.functions.len(), 1);
    assert_eq!(processor.functions[0].name, "main");

    assert_eq!(processor.enums.len(), 1);
    let e = &processor.enums[0];
    assert_eq!(e.name, "E");
    assert_eq!(e.items[0].as_ref(), "A");
    assert_eq!(e.items[1].as_ref(), "B");

    assert_eq!(processor.structures.len(), 2);
    let s = &processor.structures[0];
    assert_eq!(s.name, "S");
    assert_eq!(s.member_names, vec!["a", "b", "c", "d", "e"]);
}

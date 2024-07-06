use cmajor::{
    endpoint::EndpointDirection,
    engine::Externals,
    performer::OutputValue,
    value::types::{Primitive, Type},
    Cmajor,
};

#[test]
fn program_details() {
    let source_code = r#"
        processor Test {
            input value int in [[ hello: "world" ]];

            fn main() {
                advance();
            }
        }
    "#;

    let cmajor = Cmajor::new();
    let program = cmajor.parse(source_code).unwrap();
    let engine = cmajor
        .create_default_engine()
        .with_sample_rate(48_000)
        .build();

    let engine = engine.load(&program).unwrap();

    let program_details = engine.program_details().unwrap();
    let endpoints = program_details.endpoints().collect::<Vec<_>>();
    assert_eq!(endpoints.len(), 1);

    let input_endpoint = &endpoints[0].as_value().unwrap();
    assert_eq!(input_endpoint.id(), "in");
    assert_eq!(input_endpoint.direction(), EndpointDirection::Input);
    assert_eq!(
        input_endpoint.annotation().keys().collect::<Vec<_>>(),
        vec!["hello"]
    );
    assert_eq!(
        input_endpoint.annotation().get_str("hello").unwrap(),
        "world"
    );
    assert!(matches!(
        input_endpoint.ty(),
        Type::Primitive(Primitive::Int32)
    ));
}

#[test]
fn loading_external_variables() {
    let source_code = r#"
        processor Test {
            output value int out;

            external int value;

            fn main() {
                out <- value;
                advance();
            }
        }
    "#;

    let cmajor = Cmajor::new();
    let program = cmajor.parse(source_code).unwrap();
    let engine = cmajor
        .create_default_engine()
        .with_sample_rate(48_000)
        .build();

    let mut externals = Externals::default();
    externals.set_external_variable("Test::value", 42);

    let engine = engine
        .load_with_externals(&program, externals)
        .unwrap()
        .link()
        .unwrap();

    let mut performer = engine.performer();

    let out = performer.endpoint::<OutputValue<i32>>("out").unwrap();

    performer.advance();

    assert_eq!(out.get(), 42);
}

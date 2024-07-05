use cmajor::{
    endpoint::EndpointDirection,
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

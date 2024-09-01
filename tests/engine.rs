use cmajor::{
    endpoint::EndpointDirection,
    engine::{Engine, Error, Externals, Loaded},
    performer::{OutputValue, Performer},
    value::{
        types::{Primitive, Type},
        Complex32, ValueRef,
    },
    Cmajor,
};

#[test]
fn program_details() {
    let source_code = r#"
        processor Test {
            input value int in [[ hello: "world" ]];

            void main() {
                advance();
            }
        }
    "#;

    let cmajor = Cmajor::new();
    let program = cmajor.parse(source_code).unwrap();
    let engine = cmajor
        .create_default_engine()
        .with_sample_rate(48_000.0)
        .build();

    let engine = engine.load(&program).unwrap();

    let program_details = engine.program_details();

    assert_eq!(program_details.main_processor(), "Test");

    let endpoints = program_details.endpoints().collect::<Vec<_>>();
    assert_eq!(endpoints.len(), 1);

    let input_endpoint = &endpoints[0].as_value().unwrap();
    assert_eq!(input_endpoint.id(), "in");
    assert_eq!(input_endpoint.direction(), EndpointDirection::Input);
    assert_eq!(
        input_endpoint.annotation().keys().collect::<Vec<_>>(),
        vec!["hello"]
    );
    assert_eq!(input_endpoint.annotation().get("hello").unwrap(), "world");
    assert!(matches!(
        input_endpoint.ty(),
        Type::Primitive(Primitive::Int32)
    ));
}

fn setup<E>(
    source_code: impl AsRef<str>,
    externals: Externals,
    endpoints: impl FnOnce(&mut Engine<Loaded>) -> E,
) -> Result<(Performer, E), Error> {
    let cmajor = Cmajor::new();
    let program = cmajor.parse(source_code).unwrap();
    let engine = cmajor
        .create_default_engine()
        .with_sample_rate(48_000.0)
        .build();

    let mut engine = engine.load_with_externals(&program, externals)?;

    let endpoints = endpoints(&mut engine);

    let mut performer = engine.link()?.performer();
    performer.set_block_size(1);
    Ok((performer, endpoints))
}

#[test]
fn loading_external_variables_i32() {
    let source_code = r#"
        processor Test
        {
            output value int32 out;
            external int32 in;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, out) = setup(
        source_code,
        Externals::default().with_variable("Test::in", 42),
        |engine| engine.endpoint("out").unwrap(),
    )
    .unwrap();

    performer.advance();
    assert_eq!(performer.get::<i32>(out), 42);
}

#[test]
fn loading_external_variables_i64() {
    let source_code = r#"
        processor Test
        {
            output value int64 out;
            external int64 in;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, out) = setup(
        source_code,
        Externals::default().with_variable("Test::in", 42_i64),
        |engine| engine.endpoint("out").unwrap(),
    )
    .unwrap();

    performer.advance();
    assert_eq!(performer.get::<i64>(out), 42);
}

#[test]
fn loading_external_variables_f32() {
    let source_code = r#"
        processor Test
        {
            output value float32 out;
            external float32 in;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, out) = setup(
        source_code,
        Externals::default().with_variable("Test::in", 42_f32),
        |engine| engine.endpoint("out").unwrap(),
    )
    .unwrap();

    performer.advance();
    assert_eq!(performer.get::<f32>(out), 42_f32);
}

#[test]
fn loading_external_variables_f64() {
    let source_code = r#"
        processor Test
        {
            output value float64 out;
            external float64 in;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, out) = setup(
        source_code,
        Externals::default().with_variable("Test::in", 42_f64),
        |engine| engine.endpoint("out").unwrap(),
    )
    .unwrap();

    performer.advance();
    assert_eq!(performer.get::<f64>(out), 42_f64);
}

#[test]
fn loading_external_variables_bool() {
    let source_code = r#"
        processor Test
        {
            output value bool out;
            external bool in;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, out) = setup(
        source_code,
        Externals::default().with_variable("Test::in", true),
        |engine| engine.endpoint("out").unwrap(),
    )
    .unwrap();

    performer.advance();
    assert!(performer.get::<bool>(out));
}

#[test]
fn loading_external_variables_struct() {
    let source_code = r#"
        processor Test
        {
            output value complex32 out;
            external complex32 in;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, out) = setup(
        source_code,
        Externals::default().with_variable(
            "Test::in",
            Complex32 {
                real: 42.0,
                imag: 21.0,
            },
        ),
        |engine| engine.endpoint::<OutputValue>("out").unwrap(),
    )
    .unwrap();

    performer.advance();

    let result: Complex32 = performer.get(out).unwrap().try_into().unwrap();
    assert_eq!(result.real, 42.0);
    assert_eq!(result.imag, 21.0);
}

#[test]
fn loading_external_variables_array() {
    let source_code = r#"
        processor Test
        {
            output value int[4] out;
            external int[4] in;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, out) = setup(
        source_code,
        Externals::default().with_variable("Test::in", [1, 2, 3, 4]),
        |engine| engine.endpoint::<OutputValue>("out").unwrap(),
    )
    .unwrap();

    performer.advance();

    let value = performer.get(out).unwrap();
    let array = value.as_array().unwrap();

    assert_eq!(array.get(0), Some(ValueRef::Int32(1)));
    assert_eq!(array.get(1), Some(ValueRef::Int32(2)));
    assert_eq!(array.get(2), Some(ValueRef::Int32(3)));
    assert_eq!(array.get(3), Some(ValueRef::Int32(4)));
}

#[test]
fn loading_external_variables_are_type_checked() {
    let source_code = r#"
        processor Test
        {
            output value int32 out;
            external int32 in;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let result = setup(
        source_code,
        Externals::default().with_variable(
            "Test::in",
            Complex32 {
                real: 42.0,
                imag: 21.0,
            },
        ),
        |_| {},
    );

    assert!(result.is_err());
}

#[test]
#[should_panic(
    expected = "assertion `left == right` failed: cmajor assertion failed\n  left: 4\n right: 5"
)]
fn loading_external_functions() {
    let source_code = r#"
        namespace rust
        {
            namespace debug
            {
                external void print (bool value);
                external void print (int32 value);
                external void print (int64 value);
                external void print (float32 value);
                external void print (float64 value);
            }

            namespace test
            {
                external void assert (bool condition);
                external void assertEqual (int32 a, int32 b);
                external void assertEqual (int64 a, int64 b);
                external void assertEqual (float32 a, float32 b);
                external void assertEqual (float64 a, float64 b);
            }
        }

        processor Test
        {
            output stream float32 out;

            void main()
            {
                rust::debug::print (true);
                rust::debug::print (2147483647_i32);
                rust::debug::print (9223372036854775807_i64);
                rust::debug::print (pi);
                rust::debug::print (float32 (pi));

                rust::test::assert (true);
                rust::test::assertEqual (2 + 2, 5);

                advance();
            }
        }
    "#;

    let (mut performer, _) = setup(source_code, Externals::default(), |_| {}).unwrap();
    performer.advance();
}

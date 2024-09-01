use cmajor::{
    engine::{Engine, Loaded},
    json,
    performer::{EndpointError, InputStream, InputValue, OutputEvent, OutputValue, Performer},
    value::{
        types::{Object, Primitive, Type},
        Complex32, Complex64, Value, ValueRef,
    },
    Cmajor,
};

fn setup<E>(program: &str, endpoints: impl FnOnce(&mut Engine<Loaded>) -> E) -> (Performer, E) {
    let cmajor = Cmajor::new();

    let engine = cmajor
        .create_default_engine()
        .with_sample_rate(44_100.0)
        .build();

    let program = cmajor.parse(program).expect("failed to parse program");

    let mut engine = engine.load(&program).expect("failed to load program");

    let endpoints = endpoints(&mut engine);

    let mut performer = engine.link().unwrap().performer();
    performer.set_block_size(128);

    (performer, endpoints)
}

#[test]
fn can_read_and_write_to_value_endpoint() {
    const PROGRAM: &str = r#"
        processor Doubler
        {
            input value int int_in;
            output value int int_out;

            input value bool bool_in;
            output value bool bool_out;

            void main()
            {
                loop {
                    int_out <- int_in * 2;
                    bool_out <- bool_in;
                    advance();
                }
            }
        }
    "#;

    let (mut performer, (int_in, int_out, bool_in, bool_out)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("int_in").unwrap(),
            engine.endpoint("int_out").unwrap(),
            engine.endpoint("bool_in").unwrap(),
            engine.endpoint("bool_out").unwrap(),
        )
    });

    performer.set(int_in, 2);
    performer.set(bool_in, true);

    performer.advance();

    assert_eq!(performer.get::<i32>(int_out), 4);
    assert!(performer.get::<bool>(bool_out));
}

#[test]
fn cant_access_endpoints_with_wrong_type() {
    const PROGRAM: &str = r#"
        processor AddOne
        {
            input value float in;
            output event float out;

            void main()
            {
                out <- in + 1;
                advance();
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint::<OutputValue>("out"),
        )
    });

    assert!(matches!(
        performer.set(input, Value::Int32(5)),
        Err(EndpointError::DataTypeMismatch)
    ));

    assert!(matches!(output, Err(EndpointError::EndpointTypeMismatch)));
}

#[test]
fn can_read_and_write_complex32_numbers() {
    const PROGRAM: &str = r#"
        processor Echo
        {
            input value complex in;
            output value complex out;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint("out").unwrap(),
        )
    });

    let value: Value = Complex32 {
        real: 2.0,
        imag: 1.0,
    }
    .into();
    performer.set(input, value).unwrap();

    performer.advance();

    let result: Complex32 = performer.get::<Value>(output).unwrap().try_into().unwrap();

    assert_eq!(
        result,
        Complex32 {
            real: 2.0,
            imag: 1.0
        }
    );
}

#[test]
fn can_read_and_write_complex64_numbers() {
    const PROGRAM: &str = r#"
        processor Echo
        {
            input value complex64 in;
            output value complex64 out;

            void main()
            {
                out <- in;
                advance();
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint("out").unwrap(),
        )
    });

    let value: Value = Complex64 {
        real: 2.0,
        imag: 1.0,
    }
    .into();
    performer.set(input, value).unwrap();

    performer.advance();

    let result: Complex64 = performer.get::<Value>(output).unwrap().try_into().unwrap();

    assert_eq!(
        result,
        Complex64 {
            real: 2.0,
            imag: 1.0
        }
    );
}

#[test]
fn can_read_structs() {
    const PROGRAM: &str = r#"
        processor Echo
        {
            output value S out;

            struct S
            {
                bool a;
                float b;
                int c;
            }

            void main()
            {
                out <- S (true, 7.0, 42);
                advance();
            }
        }
    "#;

    let (mut performer, output) = setup(PROGRAM, |engine| engine.endpoint("out").unwrap());

    performer.advance();

    let value = performer.get::<Value>(output).unwrap();
    let object = value.as_object().unwrap();

    assert_eq!(object.field("a").unwrap(), ValueRef::Bool(true));
    assert_eq!(object.field("b").unwrap(), ValueRef::Float32(7.0));
    assert_eq!(object.field("c").unwrap(), ValueRef::Int32(42));
}

#[test]
fn can_read_and_write_arrays() {
    const PROGRAM: &str = r#"
        processor Reverser
        {
            input value int[4] in;
            output value int[4] out;

            void main()
            {
                out <- int[] (in[3], in[2], in[1], in[0]);
                advance();
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint("out").unwrap(),
        )
    });

    performer.set::<Value>(input, [1, 2, 3, 4].into()).unwrap();

    performer.advance();

    let value = performer.get::<Value>(output).unwrap();
    let array = value.as_array().unwrap();

    assert_eq!(array.len(), 4);

    assert_eq!(array.get(0), Some(ValueRef::Int32(4)));
    assert_eq!(array.get(1), Some(ValueRef::Int32(3)));
    assert_eq!(array.get(2), Some(ValueRef::Int32(2)));
    assert_eq!(array.get(3), Some(ValueRef::Int32(1)));
}

#[test]
fn can_post_events() {
    const PROGRAM: &str = r#"
        processor P
        {
            input event (int, bool) in;
            output value int out;

            event in (int x)
            {
                out <- x * x;
            }

            event in (bool x)
            {
                out <- x ? 42 : 0;
            }

            void main()
            {
                advance();
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint("out").unwrap(),
        )
    });

    performer.post(input, 4).unwrap();
    performer.advance();

    assert_eq!(performer.get::<i32>(output), 16);

    performer.post(input, true).unwrap();
    performer.advance();

    assert_eq!(performer.get(output), 42);
}

#[test]
fn can_read_events() {
    const PROGRAM: &str = r#"
        processor Echo
        {
            input event (int, bool) in;
            output event (int, bool) out;

            event in(int value)
            {
                out <- value;
            }

            event in(bool value)
            {
                out <- value;
            }

            void main()
            {
                advance();
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint("out").unwrap(),
        )
    });

    performer.post(input, 5).unwrap();
    performer.advance();
    assert_eq!(
        performer
            .fetch(output, |frame, data| {
                assert_eq!(frame, 0);
                assert_eq!(data, ValueRef::Int32(5));
            })
            .unwrap(),
        1
    );

    performer.post(input, true).unwrap();
    performer.advance();
    assert_eq!(
        performer
            .fetch(output, |frame, data| {
                assert_eq!(frame, 0);
                assert_eq!(data, ValueRef::Bool(true));
            })
            .unwrap(),
        1
    );
}

#[test]
fn can_read_streams() {
    const PROGRAM: &str = r#"
        processor Iota
        {
            output stream int out;

            void main()
            {
                int i = 0;
                loop {
                    out <- i;
                    i += 1;
                    advance();
                }
            }
        }
    "#;

    let (mut performer, stream) = setup(PROGRAM, |engine| engine.endpoint("out").unwrap());

    performer.set_block_size(8);

    performer.advance();

    let mut buffer = [0_i32; 8];

    performer.read(stream, buffer.as_mut_slice());
    assert_eq!(buffer, [0, 1, 2, 3, 4, 5, 6, 7]);

    performer.advance();

    performer.read(stream, buffer.as_mut_slice());
    assert_eq!(buffer, [8, 9, 10, 11, 12, 13, 14, 15]);

    assert_eq!(performer.get_xruns(), 0);
}

#[test]
fn can_query_endpoint_information() {
    const PROGRAM: &str = r#"
        processor P
        {
            input stream int a;
            input value float b;
            output event (int, S) c;

            struct S {
                bool d;
            }

            void main()
            {
                advance();
            }
        }
    "#;

    let (performer, _) = setup(PROGRAM, |engine| {
        (
            engine.endpoint::<InputStream<i32>>("a").unwrap(),
            engine.endpoint::<InputValue<f32>>("b").unwrap(),
            engine.endpoint::<OutputEvent>("c").unwrap(),
        )
    });

    let a = performer.endpoint_by_id("a").unwrap();
    let a = a.as_stream().expect("expected stream");

    assert_eq!(a.id(), "a");
    assert!(a.ty().is::<i32>());

    let b = performer.endpoint_by_id("b").unwrap();
    let b = b.as_value().expect("expected value");

    assert_eq!(b.id(), "b");
    assert!(b.ty().is::<f32>());

    let c = performer.endpoint_by_id("c").unwrap();
    let c = c.as_event().expect("expected event");

    assert_eq!(c.id(), "c");
    assert!(c.types()[0].is::<i32>());
    assert_eq!(
        c.types()[1],
        Object::new("S")
            .with_field("d", Type::Primitive(Primitive::Bool))
            .into()
    );
}

#[test]
fn can_write_streams() {
    const PROGRAM: &str = r#"
        processor Doubler
        {
            input stream int in;
            output stream int out;

            void main()
            {
                loop {
                    out <- in * 2;
                    advance();
                }
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint("out").unwrap(),
        )
    });

    let mut buffer = [1, 2, 3, 4, 5, 6, 7, 8];
    performer.set_block_size(buffer.len() as u32);

    performer.write(input, buffer.as_mut_slice());
    performer.advance();
    performer.read(output, buffer.as_mut_slice());

    assert_eq!(buffer, [2, 4, 6, 8, 10, 12, 14, 16]);
}

#[test]
fn read_and_write_vectors() {
    const PROGRAM: &str = r#"
        processor Echo
        {
            input value int<4> in;
            output value int<4> out;

            void main()
            {
                loop {
                    out <- in;
                    advance();
                }
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint("out").unwrap(),
        )
    });

    performer.set::<Value>(input, [1, 2, 3, 4].into()).unwrap();
    performer.advance();

    let value = performer.get::<Value>(output).unwrap();
    let array = value.as_array().unwrap();

    let elems: Vec<_> = array.elems().collect();
    assert_eq!(
        elems,
        vec![
            ValueRef::Int32(1),
            ValueRef::Int32(2),
            ValueRef::Int32(3),
            ValueRef::Int32(4)
        ]
    );
}

#[test]
fn endpoints_with_annotations() {
    const PROGRAM: &str = r#"
        processor P
        {
            input value float a [[ name: "foo", min: 0.5, max: 10.0, hidden: true ]];
            output value int b [[ name: "bar", min: 1, max: 5, hidden: false ]];

            void main()
            {
                advance();
            }
        }
    "#;

    let (performer, _) = setup(PROGRAM, |engine| {
        let _ = engine.endpoint::<InputValue<f32>>("a");
        let _ = engine.endpoint::<OutputValue<i32>>("b");
    });

    let a = performer.endpoint_by_id("a").unwrap();

    assert_eq!(
        a.annotation().get("name").and_then(json::Value::as_str),
        Some("foo")
    );
    assert_eq!(
        a.annotation().get("min").and_then(json::Value::as_f64),
        Some(0.5)
    );
    assert_eq!(
        a.annotation().get("max").and_then(json::Value::as_f64),
        Some(10.0)
    );
    assert_eq!(
        a.annotation().get("hidden").and_then(json::Value::as_bool),
        Some(true)
    );

    let b = performer.endpoint_by_id("b").unwrap();
    assert_eq!(
        b.annotation().get("name").and_then(json::Value::as_str),
        Some("bar")
    );
    assert_eq!(
        b.annotation().get("min").and_then(json::Value::as_i64),
        Some(1)
    );
    assert_eq!(
        b.annotation().get("max").and_then(json::Value::as_i64),
        Some(5)
    );
    assert_eq!(
        b.annotation().get("hidden").and_then(json::Value::as_bool),
        Some(false)
    );
}

#[test]
fn multiple_handles_to_the_same_input_value_endpoint() {
    const PROGRAM: &str = r#"
        processor P
        {
            input value int a;
            output value int b;

            void main()
            {
                loop {
                    b <- a;
                    advance();
                }
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (engine.endpoint("a").unwrap(), engine.endpoint("b").unwrap())
    });

    performer.set(input, 42);
    performer.advance();

    assert_eq!(performer.get::<i32>(output), 42);

    performer.set(input, 24);
    performer.advance();

    assert_eq!(performer.get(output), 24);
}

#[test]
fn multiple_handles_to_the_same_output_value_endpoint() {
    const PROGRAM: &str = r#"
        processor P
        {
            input value int a;
            output value int b;

            void main()
            {
                loop {
                    b <- a;
                    advance();
                }
            }
        }
    "#;

    let (mut performer, (a, b, c)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("a").unwrap(),
            engine.endpoint("b").unwrap(),
            engine.endpoint("b").unwrap(),
        )
    });

    performer.set(a, 42);

    performer.advance();

    assert_eq!(performer.get::<i32>(b), 42);
    assert_eq!(performer.get::<i32>(c), 42);
}

#[test]
fn void_events() {
    const PROGRAM: &str = r#"
        processor P
        {
            input event void increment;
            output value int currentCount;

            int counter = 0;

            event increment()
            {
                counter += 1;
                currentCount <- counter;
            }

            void main()
            {
                loop {
                    advance();
                }
            }
        }
    "#;

    let (mut performer, (increment, current_count)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("increment").unwrap(),
            engine.endpoint("currentCount").unwrap(),
        )
    });

    performer.post(increment, ()).unwrap();
    performer.advance();

    assert_eq!(performer.get::<i32>(current_count), 1);

    performer.post(increment, ()).unwrap();
    performer.advance();

    assert_eq!(performer.get::<i32>(current_count), 2);
}

#[test]
fn vector_stream_endpoints() {
    const PROGRAM: &str = r#"
        processor Swap
        {
            input stream float<2> in;
            output stream float<2> out;

            void main()
            {
                loop {
                    out <- float<2> (in[1], in[0]);
                    advance();
                }
            }
        }
    "#;

    let (mut performer, (input, output)) = setup(PROGRAM, |engine| {
        (
            engine.endpoint("in").unwrap(),
            engine.endpoint("out").unwrap(),
        )
    });

    let input_buffer = [[1_f32, 2_f32]; 4];
    let mut output_buffer = [[0_f32; 2]; 4];

    performer.set_block_size(4);

    performer.write(input, &input_buffer);
    performer.advance();
    performer.read(output, &mut output_buffer);

    assert_eq!(output_buffer, [[2., 1.], [2., 1.], [2., 1.], [2., 1.]]);
}

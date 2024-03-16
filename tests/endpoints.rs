use cmajor::{
    endpoint::Endpoint,
    performer::{EndpointError, Performer},
    value::{
        types::{Object, Primitive, Type},
        Complex32, Complex64, ValueRef,
    },
    Cmajor,
};

fn setup(program: &str) -> Performer {
    let cmajor = Cmajor::new();

    let llvm = cmajor
        .engine_types()
        .find(|engine_type| engine_type == "llvm")
        .expect("no llvm engine type");

    let engine = cmajor.create_engine(llvm).with_sample_rate(44_100).build();

    let program = cmajor.parse(program).expect("failed to parse program");

    let engine = engine
        .load(&program)
        .and_then(|engine| engine.link())
        .expect("failed to load and link program");

    let mut performer = engine.performer();
    performer.set_block_size(128);
    performer
}

#[test]
fn can_read_and_write_to_value_endpoint() {
    const PROGRAM: &str = r#"
        processor Doubler
        {
            input value int in;
            output value int out;
        
            void main()
            {
                out <- in * 2;
                advance();
            }
        }
    "#;

    let mut performer = setup(PROGRAM);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    performer.set_value(input, 2).unwrap();
    performer.advance();

    let result = performer.get_value(output).unwrap();

    assert_eq!(result, ValueRef::Int32(4));
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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    assert!(matches!(
        performer.set_value(input, 5),
        Err(EndpointError::DataTypeMismatch)
    ));

    assert!(matches!(
        performer.get_value(output),
        Err(EndpointError::EndpointTypeMismatch)
    ));
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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    performer
        .set_value(
            input,
            Complex32 {
                imag: 1.0,
                real: 2.0,
            },
        )
        .unwrap();

    performer.advance();

    let result: Complex32 = performer.get_value(output).unwrap().try_into().unwrap();

    assert_eq!(
        result,
        Complex32 {
            imag: 1.0,
            real: 2.0,
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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    performer
        .set_value(
            input,
            Complex64 {
                imag: 1.0,
                real: 2.0,
            },
        )
        .unwrap();

    performer.advance();

    let result: Complex64 = performer.get_value(output).unwrap().try_into().unwrap();

    assert_eq!(
        result,
        Complex64 {
            imag: 1.0,
            real: 2.0,
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

    let mut performer = setup(PROGRAM);
    let output = performer.endpoints().get_handle("out").unwrap();

    performer.advance();

    let result = performer.get_value(output).unwrap();
    let object = if let ValueRef::Object(object) = result {
        object
    } else {
        panic!("expected an object")
    };

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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    performer.set_value(input, [1, 2, 3, 4]).unwrap();

    performer.advance();

    let result = performer.get_value(output).unwrap();
    let array = if let ValueRef::Array(array) = result {
        array
    } else {
        panic!("expected an array")
    };
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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    performer.post_event(input, 4).unwrap();

    performer.advance();

    assert_eq!(performer.get_value(output).unwrap(), ValueRef::Int32(16));

    performer.post_event(input, true).unwrap();
    performer.advance();

    assert_eq!(performer.get_value(output).unwrap(), ValueRef::Int32(42));
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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    performer.post_event(input, 5_i32).unwrap();
    performer.advance();
    assert_eq!(
        performer
            .read_events(output, |frame, handle, data| {
                assert_eq!(frame, 0);
                assert_eq!(handle, output);
                assert_eq!(data, ValueRef::Int32(5));
            })
            .unwrap(),
        1
    );

    performer.post_event(input, true).unwrap();
    performer.advance();
    assert_eq!(
        performer
            .read_events(output, |frame, handle, data| {
                assert_eq!(frame, 0);
                assert_eq!(handle, output);
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

    let mut performer = setup(PROGRAM);
    performer.set_block_size(8);

    let output = performer.endpoints().get_handle("out").unwrap();

    performer.advance();

    let mut buffer = [0_i32; 8];

    unsafe { performer.read_stream_unchecked(output, buffer.as_mut_slice()) };
    assert_eq!(buffer, [0, 1, 2, 3, 4, 5, 6, 7]);

    performer.advance();

    unsafe { performer.read_stream_unchecked(output, buffer.as_mut_slice()) };
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

    let performer = setup(PROGRAM);

    let a = match performer.endpoints().get_by_id("a").unwrap() {
        (_, Endpoint::Stream(endpoint)) => endpoint,
        _ => panic!("expected a stream"),
    };

    assert_eq!(a.id(), "a");
    assert_eq!(a.ty(), &Type::Primitive(Primitive::Int32));

    let b = match performer.endpoints().get_by_id("b").unwrap() {
        (_, Endpoint::Value(endpoint)) => endpoint,
        _ => panic!("expected a value"),
    };

    assert_eq!(b.id(), "b");
    assert_eq!(b.ty(), &Type::Primitive(Primitive::Float32));

    let c = match performer.endpoints().get_by_id("c").unwrap() {
        (_, Endpoint::Event(endpoint)) => endpoint,
        _ => panic!("expected an event"),
    };

    assert_eq!(c.id(), "c");
    assert_eq!(
        c.types(),
        vec![
            Type::Primitive(Primitive::Int32),
            Object::new()
                .with_field("d", Type::Primitive(Primitive::Bool))
                .into()
        ]
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

    let mut performer = setup(PROGRAM);

    let mut buffer = [1, 2, 3, 4, 5, 6, 7, 8];
    performer.set_block_size(buffer.len() as u32);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    unsafe {
        performer.write_stream_unchecked(input, buffer.as_mut_slice());
    }
    performer.advance();
    unsafe { performer.read_stream_unchecked(output, buffer.as_mut_slice()) };

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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoints().get_handle("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    performer.set_value(input, [1, 2, 3, 4]).unwrap();
    performer.advance();

    let value = performer.get_value(output).unwrap();
    let array = value.as_array().expect("expected an array");

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

    let performer = setup(PROGRAM);

    let (_, a) = performer.endpoints().get_by_id("a").unwrap();

    assert_eq!(a.annotation().get_str("name"), Some("foo"));
    assert_eq!(a.annotation().get_f64("min"), Some(0.5));
    assert_eq!(a.annotation().get_f64("max"), Some(10.0));
    assert_eq!(a.annotation().get_bool("hidden"), Some(true));

    let (_, b) = performer.endpoints().get_by_id("b").unwrap();
    assert_eq!(b.annotation().get_str("name"), Some("bar"));
    assert_eq!(b.annotation().get_i64("min"), Some(1));
    assert_eq!(b.annotation().get_i64("max"), Some(5));
    assert_eq!(b.annotation().get_bool("hidden"), Some(false));
}

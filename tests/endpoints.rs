use cmajor::{
    performer::{EndpointError, InputEvent, InputValue, OutputValue, Performer},
    value::{
        types::{Object, Primitive, Type},
        Complex32, Complex64, Value, ValueRef,
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

    let mut performer = setup(PROGRAM);

    let int_in = performer.endpoint::<InputValue<i32>>("int_in").unwrap();
    let mut int_out = performer.endpoint::<OutputValue<i32>>("int_out").unwrap();

    let bool_in = performer.endpoint::<InputValue<bool>>("bool_in").unwrap();
    let mut bool_out = performer.endpoint::<OutputValue<bool>>("bool_out").unwrap();

    int_in.set(2);
    bool_in.set(true);

    performer.advance();

    assert_eq!(int_out.get(), 4);
    assert!(bool_out.get());
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

    let input = performer.endpoint::<InputValue>("in").unwrap();
    assert!(matches!(input.set(5), Err(EndpointError::DataTypeMismatch)));

    let output = performer.endpoint::<OutputValue>("out");
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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoint::<InputValue>("in").unwrap();
    let mut output = performer.endpoint::<OutputValue>("out").unwrap();

    input
        .set(Complex32 {
            imag: 1.0,
            real: 2.0,
        })
        .unwrap();

    performer.advance();

    let result: Complex32 = output.get().as_ref().try_into().unwrap();

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

    let input = performer.endpoint::<InputValue>("in").unwrap();
    let mut output = performer.endpoint::<OutputValue>("out").unwrap();

    input
        .set(Complex64 {
            imag: 1.0,
            real: 2.0,
        })
        .unwrap();

    performer.advance();

    let result: Complex64 = output.get().as_ref().try_into().unwrap();

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
    let mut output = performer.endpoint::<OutputValue>("out").unwrap();

    performer.advance();

    let result = output.get();
    let result = result.as_ref();
    let object = result.as_object().expect("expected an object");

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

    let input = performer.endpoint::<InputValue<Value>>("in").unwrap();
    let mut output = performer.endpoint::<OutputValue<Value>>("out").unwrap();

    input.set([1, 2, 3, 4]).unwrap();

    performer.advance();

    let result = output.get();
    let array = if let ValueRef::Array(array) = result.as_ref() {
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

    let input = performer.endpoint::<InputEvent>("in").unwrap();
    let mut output = performer.endpoint::<OutputValue<i32>>("out").unwrap();

    input.post(4).unwrap();
    performer.advance();

    assert_eq!(output.get(), 16);

    input.post(true).unwrap();
    performer.advance();

    assert_eq!(output.get(), 42);
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

    let input = performer.endpoint::<InputEvent>("in").unwrap();
    let output = performer.endpoints().get_handle("out").unwrap();

    input.post(5).unwrap();
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

    input.post(true).unwrap();
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

    let mut performer = performer.with_output_stream::<i32>("out").unwrap();

    performer.advance();

    let mut buffer = [0_i32; 8];

    performer.read_stream(buffer.as_mut_slice());
    assert_eq!(buffer, [0, 1, 2, 3, 4, 5, 6, 7]);

    performer.advance();

    performer.read_stream(buffer.as_mut_slice());
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

    let (_, a) = performer.endpoints().get_by_id("a").unwrap();
    let a = a.as_stream().expect("expected stream");

    assert_eq!(a.id(), "a");
    assert!(a.ty().is::<i32>());

    let (_, b) = performer.endpoints().get_by_id("b").unwrap();
    let b = b.as_value().expect("expected value");

    assert_eq!(b.id(), "b");
    assert!(b.ty().is::<f32>());

    let (_, c) = performer.endpoints().get_by_id("c").unwrap();
    let c = c.as_event().expect("expected event");

    assert_eq!(c.id(), "c");
    assert!(c.types()[0].is::<i32>());
    assert_eq!(
        c.types()[1],
        Object::new()
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

    let mut performer = setup(PROGRAM)
        .with_input_stream::<i32>("in")
        .unwrap()
        .with_output_stream::<i32>("out")
        .unwrap();

    let mut buffer = [1, 2, 3, 4, 5, 6, 7, 8];
    performer.set_block_size(buffer.len() as u32);

    performer.write_stream(buffer.as_mut_slice());
    performer.advance();
    performer.read_stream(buffer.as_mut_slice());

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

    let input = performer.endpoint::<InputValue>("in").unwrap();
    let mut output = performer.endpoint::<OutputValue>("out").unwrap();

    input.set([1, 2, 3, 4]).unwrap();
    performer.advance();

    let value = output.get();
    let value_ref = value.as_ref();
    let array = value_ref.as_array().expect("expected an array");

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

    let mut performer = setup(PROGRAM);

    let input_a = performer.endpoint::<InputValue<i32>>("a").unwrap();
    let input_b = performer.endpoint::<InputValue<i32>>("a").unwrap();
    let mut output = performer.endpoint::<OutputValue<i32>>("b").unwrap();

    input_a.set(42);
    performer.advance();

    assert_eq!(output.get(), 42);

    input_b.set(24);
    performer.advance();

    assert_eq!(output.get(), 24);
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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoint::<InputValue<i32>>("a").unwrap();
    let mut output_a = performer.endpoint::<OutputValue<i32>>("b").unwrap();
    let mut output_b = performer.endpoint::<OutputValue<i32>>("b").unwrap();

    input.set(42);

    performer.advance();

    assert_eq!(output_a.get(), 42);
    assert_eq!(output_b.get(), 42);
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

    let mut performer = setup(PROGRAM);

    let input = performer.endpoint::<InputEvent>("increment").unwrap();
    let mut output = performer
        .endpoint::<OutputValue<i32>>("currentCount")
        .unwrap();

    input.post(()).unwrap();
    performer.advance();

    assert_eq!(output.get(), 1);

    input.post(()).unwrap();
    performer.advance();

    assert_eq!(output.get(), 2);
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

    let mut performer = setup(PROGRAM)
        .with_input_stream::<[f32; 2]>("in")
        .unwrap()
        .with_output_stream::<[f32; 2]>("out")
        .unwrap();

    let input_buffer = [[1., 2.]; 4];
    let mut output_buffer = [[0.; 2]; 4];

    performer.set_block_size(4);

    performer.write_stream(&input_buffer);
    performer.advance();
    performer.read_stream(&mut output_buffer);

    assert_eq!(output_buffer, [[2., 1.], [2., 1.], [2., 1.], [2., 1.]]);
}

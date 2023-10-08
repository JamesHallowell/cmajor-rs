use cmajor::{
    values::{Complex32, Complex64, ValueView},
    Cmajor, EndpointHandles, Performer,
};

fn setup(program: &str) -> (Performer, EndpointHandles) {
    let cmajor = Cmajor::new("libCmajPerformer.dylib").expect("failed to load library");

    let llvm = cmajor
        .engine_types()
        .find(|engine_type| engine_type == "llvm")
        .expect("no llvm engine type");

    let engine = cmajor.create_engine(llvm).with_sample_rate(44_100).build();

    let program = cmajor.parse(program).expect("failed to parse program");

    let engine = engine.load(&program).expect("failed to load program");
    let engine = engine.link().expect("failed to link program");

    let (performer, endpoints) = engine.performer();

    (performer, endpoints)
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

    let (mut performer, mut endpoints) = setup(PROGRAM);

    let input = endpoints.get_input("in").unwrap();
    let output = performer.get_output("out").unwrap();

    endpoints.write_value(input, 2).unwrap();
    performer.advance(1);

    let result = performer.read_value(output).unwrap();

    assert!(matches!(result.get(), ValueView::Int32(4)));
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

    let (mut performer, mut endpoints) = setup(PROGRAM);

    let input = endpoints.get_input("in").unwrap();
    let output = performer.get_output("out").unwrap();

    assert!(matches!(
        endpoints.write_value(input, 5),
        Err(cmajor::EndpointError::DataTypeMismatch)
    ));

    assert!(matches!(
        performer.read_value(output),
        Err(cmajor::EndpointError::EndpointTypeMismatch)
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

    let (mut performer, mut endpoints) = setup(PROGRAM);

    let input = endpoints.get_input("in").unwrap();
    let output = performer.get_output("out").unwrap();

    endpoints
        .write_value(
            input,
            Complex32 {
                imag: 1.0,
                real: 2.0,
            },
        )
        .unwrap();

    performer.advance(1);

    let result: Complex32 = performer.read_value(output).unwrap().try_into().unwrap();

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

    let (mut performer, mut endpoints) = setup(PROGRAM);

    let input = endpoints.get_input("in").unwrap();
    let output = performer.get_output("out").unwrap();

    endpoints
        .write_value(
            input,
            Complex64 {
                imag: 1.0,
                real: 2.0,
            },
        )
        .unwrap();

    performer.advance(1);

    let result: Complex64 = performer.read_value(output).unwrap().try_into().unwrap();

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

    let (mut performer, _) = setup(PROGRAM);
    let output = performer.get_output("out").unwrap();

    performer.advance(1);

    let result = performer.read_value(output).unwrap();
    let object = result.object().unwrap();

    assert_eq!(object.field("a").unwrap(), ValueView::Bool(true));
    assert_eq!(object.field("b").unwrap(), ValueView::Float32(7.0));
    assert_eq!(object.field("c").unwrap(), ValueView::Int32(42));
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

    let (mut performer, mut endpoints) = setup(PROGRAM);

    let input = endpoints.get_input("in").unwrap();
    let output = performer.get_output("out").unwrap();

    endpoints.write_value(input, [1, 2, 3, 4]).unwrap();

    performer.advance(1);

    let result = performer.read_value(output).unwrap();
    let array = result.array().unwrap();
    assert_eq!(array.len(), 4);

    let output = array
        .into_iter()
        .map(|value| match value {
            ValueView::Int32(value) => Some(value),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(output, vec![4, 3, 2, 1]);
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

    let (mut performer, mut endpoints) = setup(PROGRAM);

    let input = endpoints.get_input("in").unwrap();
    let output = performer.get_output("out").unwrap();

    endpoints.post_event(input, 4).unwrap();

    performer.advance(1);

    assert_eq!(
        performer.read_value(output).unwrap().get(),
        ValueView::Int32(16)
    );

    endpoints.post_event(input, true).unwrap();
    performer.advance(1);

    assert_eq!(
        performer.read_value(output).unwrap().get(),
        ValueView::Int32(42)
    );
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

    let (mut performer, mut endpoints) = setup(PROGRAM);

    let input = endpoints.get_input("in").unwrap();
    let output = performer.get_output("out").unwrap();

    endpoints.post_event(input, 5_i32).unwrap();
    endpoints.post_event(input, true).unwrap();

    performer.advance(1);

    let mut events = vec![];
    performer
        .read_events(output, |frame, handle, data| {
            assert_eq!(frame, 0);
            assert_eq!(handle, output);

            events.push(data.to_owned());
        })
        .unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].get(), ValueView::Int32(5));
    assert_eq!(events[1].get(), ValueView::Bool(true));
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

    let (mut performer, _) = setup(PROGRAM);

    let output = performer.get_output("out").unwrap();

    performer.advance(8);

    let mut buffer = [0_i32; 8];

    unsafe { performer.read_stream_unchecked(output, buffer.as_mut_slice()) };
    assert_eq!(buffer, [0, 1, 2, 3, 4, 5, 6, 7]);

    performer.advance(8);

    unsafe { performer.read_stream_unchecked(output, buffer.as_mut_slice()) };
    assert_eq!(buffer, [8, 9, 10, 11, 12, 13, 14, 15]);

    assert_eq!(performer.get_xruns(), 0);
}

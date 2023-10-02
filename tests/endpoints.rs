use cmajor::{Cmajor, Complex, Complex32, Complex64, Endpoints, Performer};

fn setup(program: &str) -> (Performer, Endpoints) {
    let cmajor = Cmajor::new("libCmajPerformer.dylib").expect("failed to load library");

    let llvm = cmajor
        .engine_types()
        .find(|engine_type| engine_type == "llvm")
        .expect("no llvm engine type");

    let engine = cmajor.create_engine(llvm).with_sample_rate(44_100).build();

    let program = cmajor.parse(program).expect("failed to parse program");

    let engine = engine.load(&program).expect("failed to load program");
    let engine = engine.link().expect("failed to link program");

    let (performer, endpoints) = engine
        .performer()
        .with_block_size(256)
        .build()
        .expect("failed to build performer");

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

    endpoints.write_value("in", 2).unwrap();
    performer.advance();
    let result: i32 = performer.read_value("out").unwrap();

    assert_eq!(result, 4);
}

#[test]
fn cant_access_endpoints_with_wrong_type() {
    const PROGRAM: &str = r#"
        processor P
        {
            input value float in;
            output stream float out;
        
            void main()
            {
                out <- in + 1;
                out <- 1.0;
                advance();
            }
        }
    "#;

    let (mut performer, mut endpoints) = setup(PROGRAM);

    assert!(matches!(
        endpoints.write_value("in", 5),
        Err(cmajor::EndpointError::DataTypeMismatch)
    ));

    assert!(matches!(
        performer.read_stream("out", &mut [0_i32; 512]),
        Err(cmajor::EndpointError::DataTypeMismatch)
    ));

    assert!(matches!(
        performer.read_value::<f32>("out"),
        Err(cmajor::EndpointError::EndpointTypeMismatch)
    ));
}

#[test]
fn can_read_and_write_complex_numbers() {
    const PROGRAM: &str = r#"
        processor P
        {
            input value complex in32;
            output value complex out32;
        
            void main()
            {
                out32 <- in32;
                advance();
            }
        }
    "#;

    let (mut performer, mut endpoints) = setup(PROGRAM);

    endpoints
        .write_value(
            "in32",
            Complex32 {
                imag: 1.0,
                real: 2.0,
            },
        )
        .unwrap();

    performer.advance();

    assert_eq!(
        performer.read_value::<Complex32>("out32").unwrap(),
        Complex32 {
            imag: 1.0,
            real: 2.0
        }
    );
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

    endpoints.post_event("in", 4).unwrap();
    performer.advance();

    assert_eq!(performer.read_value::<i32>("out").unwrap(), 16);

    endpoints.post_event("in", true).unwrap();
    performer.advance();

    assert_eq!(performer.read_value::<i32>("out").unwrap(), 42);
}

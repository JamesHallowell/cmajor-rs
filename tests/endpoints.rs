use cmajor::{Cmajor, Endpoints, Performer};

fn setup(program: &str) -> (Performer, Endpoints) {
    let cmajor = Cmajor::new("libCmajPerformer.dylib").expect("failed to load library");

    let llvm = cmajor
        .engine_types()
        .find(|engine_type| engine_type == "llvm")
        .expect("no llvm engine type");

    let engine = cmajor.create_engine(llvm).with_sample_rate(48_000).build();

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
fn can_write_to_value_endpoint() {
    const PROGRAM: &str = r#"
        processor Doubler
        {
            input value int in;
            output value int doubled;
        
            void main()
            {            
                doubled <- in * 2;
                advance();
            }
        }
    "#;

    let (mut performer, mut endpoints) = setup(PROGRAM);

    endpoints.input_value("in").unwrap().send(2);
    performer.advance();
    let result = performer.output_value("doubled").unwrap().get();

    assert_eq!(result, 4);
}

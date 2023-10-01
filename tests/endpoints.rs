use cmajor::{Cmajor, Engine};

fn setup() -> (Cmajor, Engine) {
    let cmajor = Cmajor::new("libCmajPerformer.dylib").expect("failed to load library");

    let llvm = cmajor
        .engine_types()
        .find(|engine_type| engine_type == "llvm")
        .expect("no llvm engine type");

    let engine = cmajor.create_engine(llvm).with_sample_rate(48_000).build();

    (cmajor, engine)
}

#[test]
fn can_write_to_value_endpoint() {
    let (cmajor, engine) = setup();

    const PROGRAM: &str = r#"
        processor Test
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

    let mut program = cmajor.create_program();
    program.parse(PROGRAM).unwrap();

    let engine = engine.load(&program).unwrap();

    let value = engine.get_endpoint_handle("value");
    let doubled = engine.get_endpoint_handle("doubled");

    let engine = engine.link().unwrap();
    let mut performer = engine.create_performer();
}

use cmajor::{
    value::{Value, ValueRef},
    Cmajor,
};

const PROGRAM: &str = r#"
processor EndpointsExample
{
    input value int valueIn;
    output value int valueOut;
    input event (int, bool) eventIn;
    input stream float64 streamIn;
    output stream float64 streamOut;

    event eventIn (int value) {
        // Do something with the event...
    }

    event eventIn (bool value) {
        // Do something with the event...
    }

    void main()
    {
        loop {
            // Double the input and write it out
            valueOut <- valueIn * 2;
            streamOut <- streamIn * 0.5;
            advance();
        }
    }
}
"#;

const SAMPLE_RATE: f64 = 48_000.0;
const BLOCK_SIZE: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmajor = Cmajor::new_from_env()?;

    println!("Cmajor v{}", cmajor.version());

    let engine = cmajor
        .create_default_engine()
        .with_sample_rate(SAMPLE_RATE)
        .build();

    let program = cmajor.parse(PROGRAM)?;
    let mut engine = engine.load(&program)?;

    let value_in = engine.endpoint("valueIn")?;
    let value_out = engine.endpoint("valueOut")?;

    let value_in_dynamic = engine.endpoint("valueIn")?;
    let value_out_dynamic = engine.endpoint("valueOut")?;

    let event_in = engine.endpoint("eventIn")?;

    let stream_in = engine.endpoint("streamIn")?;
    let stream_out = engine.endpoint("streamOut")?;

    let mut performer = engine.link()?.performer();

    performer.set_block_size(BLOCK_SIZE);

    /*
       If you know the types of your endpoints at compile-time, then you can use the strongly-typed
       endpoints and avoid run-time type checking when reading and writing to that endpoint.
    */
    performer.set(value_in, 21);
    performer.advance();
    assert_eq!(performer.get::<i32>(value_out), 42);

    /*
        However, if you don't know the type at compile time, then you can use the generic [`Value`]
        placeholder and the types will get checked when reading and writing to the endpoint.
    */
    performer.set(value_in_dynamic, Value::Int32(7))?;
    performer.advance();
    assert_eq!(
        performer.get::<Value>(value_out_dynamic),
        Ok(ValueRef::Int32(14))
    );

    /*
       You can also use the generic [`Value`] placeholder for the event endpoints. The endpoints
       will reject types that don't match the expected type.
    */
    performer.post(event_in, true)?;
    performer.post(event_in, 42)?;

    assert!(performer.post(event_in, 12.0).is_err());

    performer.advance();

    /*
        Stream endpoints have more restrictions on the types that can be read and written.
    */
    let mut buffer = [1., 2., 3., 4.];

    performer.write(stream_in, &buffer);
    performer.advance();
    performer.read(stream_out, &mut buffer);

    assert_eq!(buffer, [0.5, 1., 1.5, 2.]);

    Ok(())
}

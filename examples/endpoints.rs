use cmajor::{
    performer::{InputEvent, InputValue, OutputValue},
    value::Value,
    Cmajor,
};

const PROGRAM: &str = r#"
processor EndpointsExample
{
    input value int valueIn;
    output value int valueOut;
    input event (int, float64<2>) eventIn;
    input stream float64 streamIn;
    output stream float64 streamOut;

    event eventIn (int value) {
        // Do something with the event...
    }

    event eventIn (float64<2> value) {
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

const SAMPLE_RATE: u32 = 48_000;
const BLOCK_SIZE: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmajor = Cmajor::new_from_env()?;

    println!("Cmajor v{}", cmajor.version());

    let engine = cmajor
        .create_default_engine()
        .with_sample_rate(SAMPLE_RATE)
        .build();

    let program = cmajor.parse(PROGRAM)?;
    let mut performer = engine.load(&program)?.link()?.performer();

    performer.set_block_size(BLOCK_SIZE);

    /*
       If you know the types of your endpoints at compile-time, then you can use the strongly-typed
       endpoints and avoid run-time type checking when reading and writing to that endpoint.
    */
    let value_in = performer.endpoint::<InputValue<i32>>("valueIn")?;
    let value_out = performer.endpoint::<OutputValue<i32>>("valueOut")?;

    value_in.set(21);
    performer.advance();
    assert_eq!(value_out.get(), 42);

    /*
        However, if you don't know the type at compile time, then you can use the generic [`Value`]
        placeholder and the types will get checked when reading and writing to the endpoint.
    */
    let value_in = performer.endpoint::<InputValue>("valueIn")?;
    let value_out = performer.endpoint::<OutputValue>("valueOut")?;

    value_in.set(7)?;
    performer.advance();
    assert_eq!(value_out.get(), Value::Int32(14));

    /*
       You can also use the generic [`Value`] placeholder for the event endpoints. The endpoints
       will reject types that don't match the expected type.
    */
    let event_in = performer.endpoint::<InputEvent>("eventIn")?;

    event_in.post([23.0, 44.0])?;
    event_in.post(42)?;

    assert!(event_in.post(true).is_err());

    performer.advance();

    /*
       A performer can also bind a pair of stream endpoints as its input and output streams.
    */
    let mut performer = performer
        .with_input_stream::<f64>("streamIn")?
        .with_output_stream::<f64>("streamOut")?;

    let input_buffer = [1., 2., 3., 4.];
    performer.write_stream(&input_buffer);

    performer.advance();

    let mut output_buffer = [0.; 4];
    performer.read_stream(&mut output_buffer);

    assert_eq!(output_buffer, [0.5, 1., 1.5, 2.]);

    Ok(())
}

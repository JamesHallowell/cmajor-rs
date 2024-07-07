use {
    cmajor::Cmajor,
    cpal::traits::{DeviceTrait, HostTrait, StreamTrait},
    std::{thread::sleep, time::Duration},
};

const PLAY_A_TUNE: &str = r#"
processor HelloWorld
{
    output stream float out;

    // This simple struct holds a note + duration for our melody
    struct Note
    {
        int pitch, length;

        void play() const
        {
            let numFrames = this.length * framesPerQuarterNote;
            let frequency  = std::notes::noteToFrequency (this.pitch);
            let phaseDelta = float (frequency * processor.period * twoPi);

            loop (numFrames)
            {
                out <- volume * sin (phase);
                phase = addModulo2Pi (phase, phaseDelta);
                advance();
            }
        }
    }

    // This is our processor's entry-point function, which is invoked
    // by the system
    void main()
    {
        let melody = Note[] ( (79, 1),  (77, 1),  (69, 2),  (71, 2),
                              (76, 1),  (74, 1),  (65, 2),  (67, 2),
                              (74, 1),  (72, 1),  (64, 2),  (67, 2),
                              (72, 4) );

        for (wrap<melody.size> i)
            melody[i].play();
    }

    // We'll define a couple of constants here to set the volume and tempo
    let framesPerQuarterNote = int (processor.frequency / 7);
    let volume = 0.15f;

    float phase;
}
"#;

const SAMPLE_RATE: u32 = 48_000;
const BLOCK_SIZE: u32 = 256;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmajor = Cmajor::new_from_env()?;

    println!("Cmajor v{}", cmajor.version());

    let engine = cmajor
        .create_default_engine()
        .with_sample_rate(SAMPLE_RATE)
        .build();

    let program = cmajor.parse(PLAY_A_TUNE)?;

    let engine = engine.load(&program)?.link()?;

    let mut performer = engine.performer();

    performer.set_block_size(BLOCK_SIZE);

    let mut performer = performer.with_output_stream::<f32>("out")?;

    let stream = cpal::default_host()
        .default_output_device()
        .expect("no output device")
        .build_output_stream(
            &cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(SAMPLE_RATE),
                buffer_size: cpal::BufferSize::Fixed(BLOCK_SIZE),
            },
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                performer.set_block_size(data.len() as u32);
                performer.advance();
                performer.read_stream(data);
            },
            |err| eprintln!("an error occurred on stream: {}", err),
            None,
        )?;

    stream.play()?;
    sleep(Duration::from_secs(5));

    Ok(())
}

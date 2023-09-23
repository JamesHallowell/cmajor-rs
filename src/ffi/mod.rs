use {
    cpal::traits::{DeviceTrait, HostTrait, StreamTrait},
    libloading,
    std::{
        ffi::{c_char, c_void, CStr, CString},
        path::Path,
        sync::Arc,
    },
};

mod engine;
mod engine_factory;
mod performer;
mod program;
mod string;

use {
    crate::ffi::engine_factory::{EngineFactory, EngineFactoryPtr},
    program::{Program, ProgramPtr},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to load library: {0}")]
    FailedToLoadLibrary(#[from] libloading::Error),
}

pub struct Library {
    // TODO: Do we need to hold on to libloading::Library? It doesn't implement Drop...?
    _library: Arc<libloading::Library>,
    entry_points: *mut EntryPoints,
}

type CMajorGetEntryPointsV9 = unsafe extern "C" fn() -> *mut c_void;

impl Library {
    pub fn load(path_to_library: impl AsRef<Path>) -> Result<Self, Error> {
        const LIBRARY_ENTRY_POINT: &[u8] = b"cmajor_getEntryPointsV9";

        let library = unsafe { libloading::Library::new(path_to_library.as_ref()) }?;
        let entry_point_fn: libloading::Symbol<CMajorGetEntryPointsV9> =
            unsafe { library.get(LIBRARY_ENTRY_POINT)? };

        let entry_points = unsafe { entry_point_fn() }.cast();

        Ok(Self {
            _library: Arc::new(library),
            entry_points,
        })
    }

    pub fn version(&self) -> &CStr {
        let vtable = unsafe { (*self.entry_points).vtable };
        let version = unsafe { ((*vtable).get_version)(self.entry_points) };
        unsafe { CStr::from_ptr(version) }
    }

    pub fn engine_types(&self) -> &CStr {
        let vtable = unsafe { (*self.entry_points).vtable };
        let engine_types = unsafe { ((*vtable).get_engine_types)(self.entry_points) };
        unsafe { CStr::from_ptr(engine_types) }
    }

    pub fn create_program(&self) -> ProgramPtr {
        unsafe {
            let vtable = (*self.entry_points).vtable;
            let program = ((*vtable).create_program)(self.entry_points);
            ProgramPtr::new(program)
        }
    }

    pub fn create_engine_factory(&self, engine_type: &CStr) -> EngineFactoryPtr {
        unsafe {
            let vtable = (*self.entry_points).vtable;
            let engine_factory =
                ((*vtable).create_engine_factory)(self.entry_points, engine_type.as_ptr());
            EngineFactoryPtr::new(engine_factory)
        }
    }
}

#[repr(C)]
struct EntryPointsVTable {
    get_version: unsafe extern "system" fn(*mut EntryPoints) -> *const c_char,
    create_program: unsafe extern "system" fn(*mut EntryPoints) -> *mut Program,
    get_engine_types: unsafe extern "system" fn(*mut EntryPoints) -> *const c_char,
    create_engine_factory:
        unsafe extern "system" fn(*mut EntryPoints, *const c_char) -> *mut EngineFactory,
}

#[repr(C)]
struct EntryPoints {
    vtable: *const EntryPointsVTable,
}

#[test]
fn lmao() {
    let library = Library::load("libCmajPerformer.dylib").unwrap();
    dbg!(library.version());
    dbg!(library.engine_types());

    let program = library.create_program();

    let contents = r#"
processor HelloWorld
{
    output stream float out;
    input value float volume;

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

    float phase;
}
    "#;

    if let Err(error) = program.parse(None, contents) {
        let json = json::stringify_pretty(json::parse(error.to_str().unwrap()).unwrap(), 4);
        panic!("Failed to parse program {:#?}", json);
    }

    let engine_factory = library.create_engine_factory(CString::new("llvm").unwrap().as_c_str());
    dbg!(engine_factory.name());

    let engine = engine_factory.create_engine(None);

    let build_settings = engine.build_settings();

    let mut build_settings = json::parse(build_settings.to_str().unwrap()).unwrap();
    build_settings
        .insert::<i32>("frequency", 48_000.into())
        .unwrap();

    let build_settings = json::stringify_pretty(build_settings, 4);
    let build_settings = CString::new(build_settings).unwrap();

    dbg!("Build settings", &build_settings);
    engine.set_build_settings(build_settings.as_c_str());

    if let Err(error) = engine.load(&program) {
        let json = json::stringify_pretty(json::parse(error.to_str().unwrap()).unwrap(), 4);
        println!("{json}");
    }

    let program_details = engine.program_details();
    if let Some(program_details) = program_details {
        let json =
            json::stringify_pretty(json::parse(program_details.to_str().unwrap()).unwrap(), 4);
        println!("{json}");
    }

    let out_endpoint = engine
        .get_endpoint_handle(CString::new("out").unwrap().as_c_str())
        .unwrap();

    let volume_endpoint = engine
        .get_endpoint_handle(CString::new("volume").unwrap().as_c_str())
        .unwrap();

    if let Err(err) = engine.link() {
        let json = json::stringify_pretty(json::parse(err.to_str().unwrap()).unwrap(), 4);
        println!("Link: {json}");
    }

    let performer = engine.create_performer();

    performer.set_block_size(256);
    performer.set_input_value(volume_endpoint, 0.1f32, 1);

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();

    let stream = device
        .build_output_stream(
            &cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(48_000),
                buffer_size: cpal::BufferSize::Fixed(256),
            },
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                performer.advance();
                performer.copy_output_frames(out_endpoint, data);
            },
            move |err| {
                eprintln!("an error occurred on stream: {}", err);
            },
            None,
        )
        .unwrap();

    stream.play().unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));

    println!("done!");
}

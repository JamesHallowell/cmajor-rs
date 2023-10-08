use {
    crate::{
        engine::{Engine, EngineBuilder, EngineType, EngineTypes},
        ffi::Library,
        program::{Program, ProgramError},
    },
    serde_json::{Map, Value},
    std::{ffi::CString, path::Path},
};

#[derive(thiserror::Error, Debug)]
pub enum LibraryError {
    #[error("Failed to load library")]
    FailedToLoadLibrary(#[from] libloading::Error),

    #[error("Engine not found")]
    EngineNotFound,
}

pub struct Cmajor {
    library: Library,
}

impl Cmajor {
    pub fn new(path_to_library: impl AsRef<Path>) -> Result<Self, LibraryError> {
        let library = Library::load(path_to_library)?;
        Ok(Self { library })
    }

    pub fn version(&self) -> &str {
        self.library.version().to_str().unwrap_or_default()
    }

    fn create_program(&self) -> Program {
        Program {
            inner: self.library.create_program(),
        }
    }

    pub fn parse(&self, cmajor_program: impl AsRef<str>) -> Result<Program, ProgramError> {
        let mut program = self.create_program();
        program.parse(cmajor_program)?;
        Ok(program)
    }

    pub fn engine_types(&self) -> impl Iterator<Item = EngineType> + '_ {
        EngineTypes::new(self.library.engine_types())
    }

    pub fn create_engine(&self, engine_type: EngineType) -> EngineBuilder {
        let engine_type = CString::new(engine_type.to_str())
            .expect("engine type should not contain a null character");

        let engine_factory = self
            .library
            .create_engine_factory(engine_type.as_c_str())
            .expect("engine factory not found");
        let engine = engine_factory.create_engine(None);

        let build_settings =
            if let Ok(Value::Object(build_settings)) = engine.get_build_settings().to_json() {
                build_settings
            } else {
                Map::new()
            };

        EngineBuilder {
            build_settings,
            engine: Engine::new(engine),
        }
    }
}

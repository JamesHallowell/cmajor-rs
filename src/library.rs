use {
    crate::{
        engine::{Engine, EngineBuilder, EngineType, EngineTypes},
        ffi::Library,
        program::Program,
        ParseError,
    },
    serde_json::{Map, Value},
    std::{ffi::CString, path::Path},
};

/// An error that can occur when loading the Cmajor library.
#[derive(thiserror::Error, Debug)]
pub enum LibraryError {
    /// Failed to load the library.
    #[error("Failed to load library")]
    FailedToLoadLibrary(#[from] libloading::Error),

    /// Failed to create an engine of the requested type.
    #[error("Engine not found")]
    EngineNotFound,

    /// The environment variable containing the path to the Cmajor library was not set.
    #[error("CMAJOR_LIB_PATH environment variable not set")]
    EnvVarNotSet,
}

/// The Cmajor library.
pub struct Cmajor {
    library: Library,
}

impl Default for Cmajor {
    fn default() -> Self {
        Self::new()
    }
}

impl Cmajor {
    /// Create a new instance of the Cmajor library.
    #[cfg(feature = "static")]
    pub fn new() -> Self {
        Self {
            library: Library::new(),
        }
    }

    /// Create a new instance of the Cmajor library.
    ///
    /// # Panics
    ///
    /// Panics if the library fails to load.
    #[cfg(not(feature = "static"))]
    pub fn new() -> Self {
        Self::new_from_env().unwrap()
    }

    /// Load the Cmajor library at the given path.
    pub fn new_from_path(path_to_library: impl AsRef<Path>) -> Result<Self, LibraryError> {
        let library = Library::load(path_to_library)?;
        Ok(Self { library })
    }

    /// Load the Cmajor library from the path specified at the `CMAJOR_LIB_PATH` environment variable.
    pub fn new_from_env() -> Result<Self, LibraryError> {
        let _ = dotenvy::dotenv();

        std::env::var("CMAJOR_LIB_PATH")
            .map_err(|_| LibraryError::EnvVarNotSet)
            .and_then(Self::new_from_path)
    }

    /// Returns the version of the Cmajor library.
    pub fn version(&self) -> &str {
        self.library.version().to_str().unwrap_or_default()
    }

    fn create_program(&self) -> Program {
        Program {
            inner: self.library.create_program(),
        }
    }

    /// Parse a Cmajor program.
    pub fn parse(&self, cmajor_program: impl AsRef<str>) -> Result<Program, ParseError> {
        let mut program = self.create_program();
        program.parse(cmajor_program)?;
        Ok(program)
    }

    /// Returns the available engine types.
    pub fn engine_types(&self) -> impl Iterator<Item = EngineType> + '_ {
        EngineTypes::new(self.library.engine_types())
    }

    /// Create the default engine type (LLVM JIT).
    pub fn create_default_engine(&self) -> EngineBuilder {
        self.create_engine(EngineType::default_engine_type())
    }

    /// Create a new engine of the given type.
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

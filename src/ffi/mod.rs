use {
    crate::ffi::engine_factory::{EngineFactory, EngineFactoryPtr},
    program::Program,
    std::{
        ffi::{c_char, c_void, CStr},
        path::Path,
    },
};

mod engine;
mod engine_factory;
mod performer;
mod program;

mod externals;
mod string;
pub(crate) mod types;

pub use {engine::EnginePtr, performer::PerformerPtr, program::ProgramPtr};

pub struct Library {
    ptr: *mut EntryPoints,
}

type CMajorGetEntryPointsV10 = unsafe extern "C" fn() -> *mut c_void;

#[cfg(feature = "static")]
extern "C" {
    fn cmajor_getEntryPointsStatic() -> *mut c_void;
}

impl Library {
    #[cfg(feature = "static")]
    pub fn new() -> Self {
        Self {
            ptr: unsafe { cmajor_getEntryPointsStatic() }.cast(),
        }
    }

    fn vtable(&self) -> &EntryPointsVTable {
        unsafe {
            self.ptr
                .as_ref()
                .and_then(|library| library.vtable.as_ref())
                .expect("failed to get vtable")
        }
    }

    pub fn load(path_to_library: impl AsRef<Path>) -> Result<Self, libloading::Error> {
        const LIBRARY_ENTRY_POINT: &[u8] = b"cmajor_getEntryPointsV10";

        let library = unsafe { libloading::Library::new(path_to_library.as_ref()) }?;
        let entry_point_fn: libloading::Symbol<CMajorGetEntryPointsV10> =
            unsafe { library.get(LIBRARY_ENTRY_POINT)? };

        Ok(Self {
            ptr: unsafe { entry_point_fn() }.cast(),
        })
    }

    pub fn version(&self) -> &CStr {
        let version = unsafe { (self.vtable().get_version)(self.ptr) };
        unsafe { CStr::from_ptr(version) }
    }

    pub fn engine_types(&self) -> &CStr {
        let engine_types = unsafe { (self.vtable().get_engine_types)(self.ptr) };
        unsafe { CStr::from_ptr(engine_types) }
    }

    pub fn create_program(&self) -> ProgramPtr {
        unsafe {
            let program = (self.vtable().create_program)(self.ptr);
            ProgramPtr::new(program)
        }
    }

    pub fn create_engine_factory(&self, engine_type: &CStr) -> Option<EngineFactoryPtr> {
        let engine_factory =
            unsafe { (self.vtable().create_engine_factory)(self.ptr, engine_type.as_ptr()) };

        if engine_factory.is_null() {
            return None;
        }

        Some(EngineFactoryPtr::new(engine_factory))
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

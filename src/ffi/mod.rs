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
    entry_points: *mut EntryPoints,
}

type CMajorGetEntryPointsV10 = unsafe extern "C" fn() -> *mut c_void;

#[cfg(feature = "static")]
extern "C" {
    fn cmajor_getEntryPointsStatic() -> *mut c_void;
}

impl Library {
    #[cfg(feature = "static")]
    pub fn new() -> Self {
        let entry_points = unsafe { cmajor_getEntryPointsStatic() }.cast();
        Self { entry_points }
    }

    pub fn load(path_to_library: impl AsRef<Path>) -> Result<Self, libloading::Error> {
        const LIBRARY_ENTRY_POINT: &[u8] = b"cmajor_getEntryPointsV10";

        let library = unsafe { libloading::Library::new(path_to_library.as_ref()) }?;
        let entry_point_fn: libloading::Symbol<CMajorGetEntryPointsV10> =
            unsafe { library.get(LIBRARY_ENTRY_POINT)? };

        let entry_points = unsafe { entry_point_fn() }.cast();

        Ok(Self { entry_points })
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

    pub fn create_engine_factory(&self, engine_type: &CStr) -> Option<EngineFactoryPtr> {
        unsafe {
            let vtable = (*self.entry_points).vtable;
            let engine_factory =
                ((*vtable).create_engine_factory)(self.entry_points, engine_type.as_ptr());

            (!engine_factory.is_null()).then(|| EngineFactoryPtr::new(engine_factory))
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

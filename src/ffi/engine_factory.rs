use {
    crate::ffi::engine::EnginePtr,
    std::{
        ffi::{c_char, c_int, c_void, CStr},
        ptr::null,
    },
};

#[repr(C)]
struct EngineFactoryVTable {
    add_ref: unsafe extern "system" fn(*mut EngineFactory) -> c_int,
    release: unsafe extern "system" fn(*mut EngineFactory) -> c_int,
    ref_count: unsafe extern "system" fn(*const EngineFactory) -> c_int,
    create_engine: unsafe extern "system" fn(*mut EngineFactory, *const c_char) -> *mut c_void,
    get_name: unsafe extern "system" fn(*mut EngineFactory) -> *const c_char,
}

#[repr(C)]
pub struct EngineFactory {
    vtable: *const EngineFactoryVTable,
}

pub struct EngineFactoryPtr(*mut EngineFactory);

impl EngineFactoryPtr {
    pub fn new(engine_factory: *mut EngineFactory) -> Self {
        Self(engine_factory)
    }

    pub fn create_engine(&self, options: Option<&CStr>) -> EnginePtr {
        let options = options.map(CStr::as_ptr).unwrap_or(null());

        let engine = unsafe { ((*(*self.0).vtable).create_engine)(self.0, options) };
        EnginePtr::new(engine.cast())
    }
}

impl Drop for EngineFactoryPtr {
    fn drop(&mut self) {
        unsafe { ((*(*self.0).vtable).release)(self.0) };
    }
}

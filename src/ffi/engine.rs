use {
    crate::ffi::{
        performer::{Performer, PerformerPtr},
        program::{Program, ProgramPtr},
        string::{CMajorString, CMajorStringPtr},
    },
    std::{
        ffi::{c_char, c_int, c_void, CStr},
        ptr::{addr_of_mut, null_mut},
    },
};

type RequestExternalVariableCallback = unsafe extern "system" fn(*mut c_void, *const c_char);

type RequestExternalFunctionCallback =
    unsafe extern "system" fn(*mut c_void, *const c_char, *const c_char) -> *mut c_void;

pub type EndpointHandle = u32;

#[repr(C)]
struct EngineVTable {
    add_ref: unsafe extern "system" fn(*mut Engine) -> c_int,
    release: unsafe extern "system" fn(*mut Engine) -> c_int,
    ref_count: unsafe extern "system" fn(*const Engine) -> c_int,
    get_build_settings: unsafe extern "system" fn(*mut Engine) -> *mut CMajorString,
    set_build_settings: unsafe extern "system" fn(*mut Engine, *const c_char),
    load: unsafe extern "system" fn(
        *mut Engine,
        *mut Program,
        *mut c_void,
        RequestExternalVariableCallback,
        *mut c_void,
        RequestExternalFunctionCallback,
    ) -> *mut CMajorString,
    set_external_variable:
        unsafe extern "system" fn(*mut Engine, *const c_char, *const c_void, isize),
    unload: unsafe extern "system" fn(*mut Engine),
    get_program_details: unsafe extern "system" fn(*mut Engine) -> *mut CMajorString,
    get_endpoint_handle: unsafe extern "system" fn(*mut Engine, *const c_char) -> EndpointHandle,
    link: unsafe extern "system" fn(*mut Engine, *mut c_void) -> *mut CMajorString,
    create_performer: unsafe extern "system" fn(*mut Engine) -> *mut Performer,
    get_last_build_log: unsafe extern "system" fn(*mut Engine) -> *mut CMajorString,
}

#[repr(C)]
pub struct Engine {
    vtable: *const EngineVTable,
}

pub struct EnginePtr {
    engine: *mut Engine,
}

impl EnginePtr {
    pub fn new(engine: *mut Engine) -> Self {
        Self { engine }
    }

    pub fn build_settings(&self) -> CMajorStringPtr {
        let result = unsafe { ((*(*self.engine).vtable).get_build_settings)(self.engine) };
        unsafe { CMajorStringPtr::new(result) }
    }

    pub fn set_build_settings(&self, build_settings: &CStr) {
        unsafe {
            ((*(*self.engine).vtable).set_build_settings)(self.engine, build_settings.as_ptr())
        };
    }

    pub fn load(&self, program: &ProgramPtr) -> Result<(), CMajorStringPtr> {
        extern "system" fn request_external_variable_callback(
            ctx: *mut c_void,
            name: *const c_char,
        ) {
            println!("request_external_variable_callback: {:?} {:?}", ctx, name);
        }
        extern "system" fn request_external_function_callback(
            ctx: *mut c_void,
            name: *const c_char,
            signature: *const c_char,
        ) -> *mut c_void {
            println!(
                "request_external_function_callback: {:?} {:?} {:?}",
                ctx, name, signature
            );

            null_mut()
        }

        let error = unsafe {
            ((*(*self.engine).vtable).load)(
                self.engine,
                program.get(),
                null_mut(),
                request_external_variable_callback,
                null_mut(),
                request_external_function_callback,
            )
        };

        if error.is_null() {
            return Ok(());
        }

        Err(unsafe { CMajorStringPtr::new(error) })
    }

    pub fn program_details(&self) -> Option<CMajorStringPtr> {
        let result = unsafe { ((*(*self.engine).vtable).get_program_details)(self.engine) };

        if !result.is_null() {
            Some(unsafe { CMajorStringPtr::new(result) })
        } else {
            None
        }
    }

    pub fn get_endpoint_handle(&self, id: &CStr) -> Option<EndpointHandle> {
        let handle =
            unsafe { ((*(*self.engine).vtable).get_endpoint_handle)(self.engine, id.as_ptr()) };

        if handle != 0 {
            Some(handle)
        } else {
            None
        }
    }

    pub fn link(&self) -> Result<(), CMajorStringPtr> {
        let cache_database = null_mut();
        let error = unsafe { ((*(*self.engine).vtable).link)(self.engine, cache_database) };

        if error.is_null() {
            Ok(())
        } else {
            Err(unsafe { CMajorStringPtr::new(error) })
        }
    }

    pub fn create_performer(&self) -> PerformerPtr {
        let performer = unsafe { ((*(*self.engine).vtable).create_performer)(self.engine) };
        unsafe { PerformerPtr::new(performer) }
    }
}

impl Drop for EnginePtr {
    fn drop(&mut self) {
        unsafe { ((*(*self.engine).vtable).release)(self.engine) };
    }
}

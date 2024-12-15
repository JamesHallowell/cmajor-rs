use {
    crate::{
        endpoint::EndpointHandle,
        engine::Externals,
        ffi::{
            externals::get_external_function,
            performer::{Performer, PerformerPtr},
            program::{Program, ProgramPtr},
            string::{CmajorString, CmajorStringPtr},
            types::TypeDescription,
        },
        value::{
            types::{Primitive, Type},
            Value,
        },
    },
    serde::Deserialize,
    serde_json as json,
    std::{
        ffi::{c_char, c_int, c_void, CStr, CString},
        ptr::null_mut,
    },
};

type RequestExternalVariableCallback = unsafe extern "system" fn(*mut c_void, *const c_char);

#[derive(Debug, Deserialize)]
struct RequestExternalVariableArgs {
    name: String,
}

type RequestExternalFunctionCallback =
    unsafe extern "system" fn(*mut c_void, *const c_char, *const c_char) -> *mut c_void;

#[repr(C)]
struct EngineVTable {
    add_ref: unsafe extern "system" fn(*mut Engine) -> c_int,
    release: unsafe extern "system" fn(*mut Engine) -> c_int,
    ref_count: unsafe extern "system" fn(*const Engine) -> c_int,
    get_build_settings: unsafe extern "system" fn(*mut Engine) -> *mut CmajorString,
    set_build_settings: unsafe extern "system" fn(*mut Engine, *const c_char),
    load: unsafe extern "system" fn(
        *mut Engine,
        *mut Program,
        *mut c_void,
        RequestExternalVariableCallback,
        *mut c_void,
        RequestExternalFunctionCallback,
    ) -> *mut CmajorString,
    set_external_variable:
        unsafe extern "system" fn(*mut Engine, *const c_char, *const c_void, isize),
    unload: unsafe extern "system" fn(*mut Engine),
    get_program_details: unsafe extern "system" fn(*mut Engine) -> *mut CmajorString,
    get_endpoint_handle: unsafe extern "system" fn(*mut Engine, *const c_char) -> u32,
    link: unsafe extern "system" fn(*mut Engine, *mut c_void) -> *mut CmajorString,
    create_performer: unsafe extern "system" fn(*mut Engine) -> *mut Performer,
    get_last_build_log: unsafe extern "system" fn(*mut Engine) -> *mut CmajorString,
}

#[repr(C)]
pub struct Engine {
    vtable: *const EngineVTable,
}

#[derive(Debug)]
pub struct EnginePtr {
    engine: *mut Engine,
}

impl EnginePtr {
    pub fn new(engine: *mut Engine) -> Self {
        Self { engine }
    }

    pub fn set_build_settings(&self, build_settings: &CStr) {
        unsafe {
            ((*(*self.engine).vtable).set_build_settings)(self.engine, build_settings.as_ptr())
        };
    }

    pub fn load(&self, program: &ProgramPtr, externals: Externals) -> Result<(), CmajorStringPtr> {
        let mut ctx = LoadContext {
            engine: self.clone(),
            externals,
        };
        let ctx_ptr = std::ptr::addr_of_mut!(ctx);

        let error = unsafe {
            ((*(*self.engine).vtable).load)(
                self.engine,
                program.get(),
                ctx_ptr.cast(),
                request_external_variable_callback,
                ctx_ptr.cast(),
                request_external_function_callback,
            )
        };

        if error.is_null() {
            return Ok(());
        }

        Err(unsafe { CmajorStringPtr::new(error) })
    }

    pub fn unload(&self) {
        unsafe { ((*(*self.engine).vtable).unload)(self.engine) };
    }

    pub fn program_details(&self) -> Option<CmajorStringPtr> {
        let result = unsafe { ((*(*self.engine).vtable).get_program_details)(self.engine) };

        if !result.is_null() {
            Some(unsafe { CmajorStringPtr::new(result) })
        } else {
            None
        }
    }

    pub fn get_endpoint_handle(&self, id: &CStr) -> Option<EndpointHandle> {
        let handle =
            unsafe { ((*(*self.engine).vtable).get_endpoint_handle)(self.engine, id.as_ptr()) };

        if handle != 0 {
            Some(handle.into())
        } else {
            None
        }
    }

    pub fn link(&self) -> Result<(), CmajorStringPtr> {
        let cache_database = null_mut();
        let error = unsafe { ((*(*self.engine).vtable).link)(self.engine, cache_database) };

        if error.is_null() {
            Ok(())
        } else {
            Err(unsafe { CmajorStringPtr::new(error) })
        }
    }

    pub fn create_performer(&self) -> PerformerPtr {
        let performer = unsafe { ((*(*self.engine).vtable).create_performer)(self.engine) };
        unsafe { PerformerPtr::new(performer) }
    }

    fn set_external_variable(&self, name: &str, value: &Value) {
        let name = if let Ok(name) = CString::new(name) {
            name
        } else {
            return;
        };

        let serialised = value.serialise_as_choc_value();

        unsafe {
            ((*(*self.engine).vtable).set_external_variable)(
                self.engine,
                name.as_ptr(),
                serialised.as_ptr().cast(),
                serialised.len() as isize,
            )
        };
    }
}

impl Clone for EnginePtr {
    fn clone(&self) -> Self {
        unsafe { ((*(*self.engine).vtable).add_ref)(self.engine) };
        Self {
            engine: self.engine,
        }
    }
}

impl Drop for EnginePtr {
    fn drop(&mut self) {
        unsafe { ((*(*self.engine).vtable).release)(self.engine) };
    }
}

struct LoadContext {
    engine: EnginePtr,
    externals: Externals,
}

extern "system" fn request_external_variable_callback(ctx: *mut c_void, args: *const c_char) {
    let args = unsafe { CStr::from_ptr(args) };
    let args = match args
        .to_str()
        .map(json::from_str::<RequestExternalVariableArgs>)
    {
        Ok(Ok(details)) => details,
        Ok(Err(err)) => {
            eprintln!("request_external_variable_callback: {err:?}");
            return;
        }
        Err(err) => {
            eprintln!("request_external_variable_callback: {err:?}");
            return;
        }
    };

    let ctx = unsafe { &mut *(ctx as *mut LoadContext) };

    if let Some(value) = ctx.externals.variables.get(args.name.as_str()) {
        ctx.engine.set_external_variable(args.name.as_str(), value);
    }
}

extern "system" fn request_external_function_callback(
    _ctx: *mut c_void,
    name: *const c_char,
    signature: *const c_char,
) -> *mut c_void {
    let name = unsafe { CStr::from_ptr(name) };
    let signature = unsafe { CStr::from_ptr(signature) };
    let name = name.to_str().expect("failed to parse function symbol name");

    if let Ok(signature) = parse_function_signature(signature) {
        return get_external_function(name, signature.as_slice());
    }

    null_mut()
}

fn parse_function_signature(string: &CStr) -> Result<Vec<Primitive>, Box<dyn std::error::Error>> {
    let type_descriptions: Vec<TypeDescription> = json::from_str(string.to_str()?)?;
    type_descriptions
        .iter()
        .map(Type::try_from)
        .map(|ty| -> Result<Primitive, Box<dyn std::error::Error>> {
            ty.map(|ty| ty.as_primitive().ok_or("expected a primitive type".into()))?
        })
        .collect::<Result<Vec<_>, _>>()
}

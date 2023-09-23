use {
    crate::ffi::string::{CMajorString, CMajorStringPtr},
    std::{
        ffi::{c_char, c_int, CStr},
        ptr::null,
    },
};

#[repr(C)]
struct SyntaxTreeOptions {
    namespace_or_module: *const c_char,
    include_source_locations: bool,
    include_comments: bool,
    include_function_contents: bool,
}

#[repr(C)]
struct ProgramVTable {
    add_ref: unsafe extern "system" fn(*mut Program) -> c_int,
    release: unsafe extern "system" fn(*mut Program) -> c_int,
    ref_count: unsafe extern "system" fn(*const Program) -> c_int,
    parse: unsafe extern "system" fn(
        *mut Program,
        *const c_char,
        *const c_char,
        isize,
    ) -> *mut CMajorString,
    get_syntax_tree:
        unsafe extern "system" fn(*mut Program, *const SyntaxTreeOptions) -> *mut CMajorString,
}

#[repr(C)]
pub struct Program {
    vtable: *const ProgramVTable,
}

pub struct ProgramPtr {
    program: *mut Program,
}

impl ProgramPtr {
    pub unsafe fn new(program: *mut Program) -> Self {
        Self { program }
    }

    pub fn get(&self) -> *mut Program {
        self.program
    }

    pub fn parse(
        &self,
        file_name: Option<&CStr>,
        file_content: impl AsRef<str>,
    ) -> Result<(), CMajorStringPtr> {
        let file_name = file_name.map(CStr::as_ptr).unwrap_or(null());

        let file_content_len = file_content.as_ref().len() as isize;
        let file_content = file_content.as_ref().as_ptr().cast();

        let error = unsafe {
            ((*(*self.program).vtable).parse)(
                self.program,
                file_name,
                file_content,
                file_content_len,
            )
        };

        if error.is_null() {
            return Ok(());
        }

        Err(unsafe { CMajorStringPtr::new(error) })
    }

    pub fn syntax_tree(&self) -> CMajorStringPtr {
        let options = SyntaxTreeOptions {
            namespace_or_module: null(),
            include_source_locations: false,
            include_comments: false,
            include_function_contents: false,
        };

        let result = unsafe { ((*(*self.program).vtable).get_syntax_tree)(self.program, &options) };

        unsafe { CMajorStringPtr::new(result) }
    }
}

impl Drop for ProgramPtr {
    fn drop(&mut self) {
        unsafe { ((*(*self.program).vtable).release)(self.program) };
    }
}

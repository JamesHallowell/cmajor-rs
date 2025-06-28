use {
    crate::ffi::string::{CmajorString, CmajorStringPtr},
    std::{
        ffi::{c_char, c_int, CString},
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
    ) -> *mut CmajorString,
    get_syntax_tree:
        unsafe extern "system" fn(*mut Program, *const SyntaxTreeOptions) -> *mut CmajorString,
}

#[repr(C)]
pub struct Program {
    vtable: *const ProgramVTable,
}

#[derive(Debug)]
pub struct ProgramPtr {
    ptr: *mut Program,
}

impl ProgramPtr {
    pub(super) unsafe fn new(program: *mut Program) -> Self {
        Self { ptr: program }
    }

    fn vtable(&self) -> &ProgramVTable {
        unsafe {
            self.ptr
                .as_ref()
                .and_then(|program| program.vtable.as_ref())
                .expect("failed to get vtable")
        }
    }

    pub fn get(&self) -> *mut Program {
        self.ptr
    }

    pub fn parse(
        &self,
        file_name: Option<impl AsRef<str>>,
        file_content: impl AsRef<str>,
    ) -> Result<(), CmajorStringPtr> {
        let file_name = file_name.map(|file_name| {
            CString::new(file_name.as_ref()).expect("string should not contain a null byte")
        });
        let file_name = file_name
            .as_ref()
            .map(|file_name| file_name.as_ptr())
            .unwrap_or(null());

        let file_content = file_content.as_ref();
        let file_content_len = file_content.len() as isize;
        let file_content = file_content.as_ptr().cast();

        let error =
            unsafe { (self.vtable().parse)(self.ptr, file_name, file_content, file_content_len) };

        if error.is_null() {
            return Ok(());
        }

        Err(unsafe { CmajorStringPtr::new(error) })
    }

    pub fn get_syntax_tree(&self) -> CmajorStringPtr {
        let options = SyntaxTreeOptions {
            namespace_or_module: null(),
            include_source_locations: false,
            include_comments: false,
            include_function_contents: false,
        };

        let syntax_tree = unsafe { (self.vtable().get_syntax_tree)(self.ptr, &options) };
        assert!(!syntax_tree.is_null());

        unsafe { CmajorStringPtr::new(syntax_tree) }
    }
}

impl Drop for ProgramPtr {
    fn drop(&mut self) {
        unsafe { (self.vtable().release)(self.ptr) };
    }
}

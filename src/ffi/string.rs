use std::{
    borrow::Cow,
    ffi::{c_char, c_int},
    ptr::null_mut,
};

#[repr(C)]
struct CmajorStringVTable {
    add_ref: unsafe extern "system" fn(*mut CmajorString) -> c_int,
    release: unsafe extern "system" fn(*mut CmajorString) -> c_int,
    ref_count: unsafe extern "system" fn(*const CmajorString) -> c_int,
    begin: unsafe extern "system" fn(*const CmajorString) -> *const c_char,
    end: unsafe extern "system" fn(*const CmajorString) -> *const c_char,
}

#[repr(C)]
pub struct CmajorString {
    vtable: *const CmajorStringVTable,
}

pub struct CmajorStringPtr {
    string: *mut CmajorString,
}

impl Drop for CmajorStringPtr {
    fn drop(&mut self) {
        unsafe { ((*(*self.string).vtable).release)(self.string) };
    }
}

impl CmajorStringPtr {
    pub unsafe fn new(string: *mut CmajorString) -> Self {
        assert_ne!(string, null_mut());
        Self { string }
    }

    pub fn to_string(&self) -> Cow<'_, str> {
        let begin = unsafe { ((*(*self.string).vtable).begin)(self.string) };
        let end = unsafe { ((*(*self.string).vtable).end)(self.string) };

        let len = unsafe { end.offset_from(begin) };
        assert!(len >= 0);

        let slice = unsafe { std::slice::from_raw_parts(begin.cast(), len as usize) };

        String::from_utf8_lossy(slice)
    }
}

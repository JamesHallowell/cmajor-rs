use std::{
    ffi::{c_char, c_int},
    ptr::null_mut,
    str::Utf8Error,
};

#[repr(C)]
struct CMajorStringVTable {
    add_ref: unsafe extern "system" fn(*mut CMajorString) -> c_int,
    release: unsafe extern "system" fn(*mut CMajorString) -> c_int,
    ref_count: unsafe extern "system" fn(*const CMajorString) -> c_int,
    begin: unsafe extern "system" fn(*const CMajorString) -> *const c_char,
    end: unsafe extern "system" fn(*const CMajorString) -> *const c_char,
}

#[repr(C)]
pub struct CMajorString {
    vtable: *const CMajorStringVTable,
}

pub struct CMajorStringPtr(*mut CMajorString);

impl Drop for CMajorStringPtr {
    fn drop(&mut self) {
        unsafe { ((*(*self.0).vtable).release)(self.0) };
    }
}

impl CMajorStringPtr {
    pub unsafe fn new(string: *mut CMajorString) -> Self {
        assert_ne!(string, null_mut());
        Self(string)
    }

    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        let begin = unsafe { ((*(*self.0).vtable).begin)(self.0) };
        let end = unsafe { ((*(*self.0).vtable).end)(self.0) };

        let len = unsafe { end.offset_from(begin) };
        assert!(len >= 0);

        let slice = unsafe { std::slice::from_raw_parts(begin.cast(), len as usize) };

        std::str::from_utf8(slice)
    }
}

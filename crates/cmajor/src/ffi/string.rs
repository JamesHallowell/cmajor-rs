use std::{
    ffi::{c_char, c_int},
    ptr::null_mut,
    slice, str,
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
pub(super) struct CmajorString {
    vtable: *const CmajorStringVTable,
}

pub struct CmajorStringPtr {
    ptr: *mut CmajorString,
}

impl Drop for CmajorStringPtr {
    fn drop(&mut self) {
        unsafe { (self.vtable().release)(self.ptr) };
    }
}

impl CmajorStringPtr {
    pub(super) unsafe fn new(string: *mut CmajorString) -> Self {
        assert_ne!(string, null_mut());
        Self { ptr: string }
    }

    fn vtable(&self) -> &CmajorStringVTable {
        unsafe {
            self.ptr
                .as_ref()
                .and_then(|string| string.vtable.as_ref())
                .expect("failed to get vtable")
        }
    }

    pub fn to_str(&self) -> &str {
        let begin = unsafe { (self.vtable().begin)(self.ptr) };
        let end = unsafe { (self.vtable().end)(self.ptr) };
        let length: usize = unsafe { end.offset_from(begin) }
            .try_into()
            .expect("length should not be negative");

        let slice = unsafe { slice::from_raw_parts(begin.cast(), length) };
        str::from_utf8(slice).expect("string should be valid utf-8")
    }
}

use std::{
    borrow::Cow,
    ffi::{c_char, c_int},
    ptr::null_mut,
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

pub struct CMajorStringPtr {
    string: *mut CMajorString,
}

impl Drop for CMajorStringPtr {
    fn drop(&mut self) {
        unsafe { ((*(*self.string).vtable).release)(self.string) };
    }
}

impl CMajorStringPtr {
    pub unsafe fn new(string: *mut CMajorString) -> Self {
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

    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.to_string())
    }
}

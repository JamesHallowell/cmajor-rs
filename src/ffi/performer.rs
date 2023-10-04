use {
    crate::engine::EndpointHandle,
    std::{
        ffi::{c_char, c_double, c_int, c_void},
        ptr::null_mut,
    },
};

type HandleOutputEventCallback =
    unsafe extern "system" fn(*mut c_void, u32, u32, u32, *const c_void, u32);

#[repr(C)]
struct PerformerVTable {
    add_ref: unsafe extern "system" fn(*mut Performer) -> c_int,
    release: unsafe extern "system" fn(*mut Performer) -> c_int,
    ref_count: unsafe extern "system" fn(*const Performer) -> c_int,

    set_block_size: unsafe extern "system" fn(*mut Performer, u32),

    set_input_frames: unsafe extern "system" fn(*mut Performer, u32, *const c_void, u32),
    set_input_value: unsafe extern "system" fn(*mut Performer, u32, *const c_void, u32),
    add_input_event: unsafe extern "system" fn(*mut Performer, u32, u32, *const c_void),

    copy_output_value: unsafe extern "system" fn(*mut Performer, u32, *mut c_void),
    copy_output_frames: unsafe extern "system" fn(*mut Performer, u32, *mut c_void, u32),
    iterate_output_events:
        unsafe extern "system" fn(*mut Performer, u32, *mut c_void, HandleOutputEventCallback),

    advance: unsafe extern "system" fn(*mut Performer),
    get_string_for_handle:
        unsafe extern "system" fn(*mut Performer, u32, *mut isize) -> *const c_char,
    get_xruns: unsafe extern "system" fn(*mut Performer) -> u32,
    get_max_block_size: unsafe extern "system" fn(*mut Performer) -> u32,
    get_event_buffer_size: unsafe extern "system" fn(*mut Performer) -> u32,
    get_latency: unsafe extern "system" fn(*mut Performer) -> c_double,
}

#[repr(C)]
pub struct Performer {
    vtable: *const PerformerVTable,
}

unsafe impl Send for PerformerPtr {}

pub struct PerformerPtr {
    performer: *mut Performer,
}

impl PerformerPtr {
    pub unsafe fn new(performer: *mut Performer) -> Self {
        assert_ne!(performer, null_mut());
        Self { performer }
    }

    pub fn set_block_size(&self, block_size: u32) {
        unsafe { ((*(*self.performer).vtable).set_block_size)(self.performer, block_size) };
    }

    pub fn set_input_value(
        &self,
        handle: EndpointHandle,
        value: &[u8],
        num_frames_to_reach_value: u32,
    ) {
        let value_ptr = value.as_ptr().cast();

        unsafe {
            ((*(*self.performer).vtable).set_input_value)(
                self.performer,
                handle.into(),
                value_ptr,
                num_frames_to_reach_value,
            )
        };
    }

    pub fn add_input_event(&self, handle: EndpointHandle, type_index: u32, data: &[u8]) {
        let data_ptr = data.as_ptr().cast();

        unsafe {
            ((*(*self.performer).vtable).add_input_event)(
                self.performer,
                handle.into(),
                type_index,
                data_ptr,
            )
        };
    }

    pub fn advance(&self) {
        unsafe { ((*(*self.performer).vtable).advance)(self.performer) };
    }

    pub fn copy_output_frames<T>(&self, handle: EndpointHandle, frames: &mut [T]) {
        let num_frames = frames.len() as u32;
        let frames = frames.as_mut_ptr().cast();

        unsafe {
            ((*(*self.performer).vtable).copy_output_frames)(
                self.performer,
                handle.into(),
                frames,
                num_frames,
            )
        };
    }

    pub unsafe fn copy_output_value(&self, handle: EndpointHandle, buffer: &mut [u8]) {
        let buffer = buffer.as_mut_ptr().cast();

        unsafe {
            ((*(*self.performer).vtable).copy_output_value)(self.performer, handle.into(), buffer)
        };
    }
}

impl Drop for PerformerPtr {
    fn drop(&mut self) {
        unsafe { ((*(*self.performer).vtable).release)(self.performer) };
    }
}

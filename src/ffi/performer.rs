use {
    crate::{
        endpoint::{EndpointHandle, EndpointTypeIndex},
        ffi::externals::check_for_panic,
    },
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

    reset: unsafe extern "system" fn(*mut Performer),
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
    ptr: *mut Performer,
}

impl PerformerPtr {
    pub unsafe fn new(performer: *mut Performer) -> Self {
        assert_ne!(performer, null_mut());
        Self { ptr: performer }
    }

    fn vtable(&self) -> &PerformerVTable {
        unsafe {
            self.ptr
                .as_ref()
                .and_then(|performer| performer.vtable.as_ref())
                .expect("failed to get vtable")
        }
    }

    pub fn set_block_size(&self, block_size: u32) {
        unsafe { (self.vtable().set_block_size)(self.ptr, block_size) };
    }

    pub unsafe fn set_input_value<T>(
        &self,
        handle: EndpointHandle,
        value: *const T,
        num_frames_to_reach_value: u32,
    ) {
        let value = value.cast();
        let handle = handle.into();

        unsafe {
            (self.vtable().set_input_value)(self.ptr, handle, value, num_frames_to_reach_value)
        };
    }

    pub fn add_input_event(
        &self,
        handle: EndpointHandle,
        type_index: EndpointTypeIndex,
        data: &[u8],
    ) {
        let data_ptr = data.as_ptr().cast();
        let handle = handle.into();
        let type_index = usize::from(type_index) as u32;

        unsafe { (self.vtable().add_input_event)(self.ptr, handle, type_index, data_ptr) };
    }

    pub fn advance(&self) {
        unsafe { (self.vtable().advance)(self.ptr) };
        check_for_panic();
    }

    pub unsafe fn set_input_frames<T>(&self, handle: EndpointHandle, frames: &[T])
    where
        T: Copy,
    {
        let handle = handle.into();
        let num_frames = frames.len() as u32;
        let frames = frames.as_ptr().cast();

        (self.vtable().set_input_frames)(self.ptr, handle, frames, num_frames);
    }

    pub unsafe fn copy_output_frames<T>(&self, handle: EndpointHandle, frames: &mut [T])
    where
        T: Copy,
    {
        let handle = handle.into();
        let num_frames = frames.len() as u32;
        let frames = frames.as_mut_ptr().cast();

        (self.vtable().copy_output_frames)(self.ptr, handle, frames, num_frames);
    }

    pub fn copy_output_value(&self, handle: EndpointHandle, buffer: &mut [u8]) {
        let handle = handle.into();
        let buffer = buffer.as_mut_ptr().cast();

        unsafe { (self.vtable().copy_output_value)(self.ptr, handle, buffer) };
    }

    pub fn iterate_output_events<F>(&self, endpoint: EndpointHandle, mut callback: F)
    where
        F: FnMut(usize, EndpointHandle, EndpointTypeIndex, &[u8]),
    {
        extern "system" fn trampoline<F>(
            context: *mut c_void,
            endpoint: u32,
            type_index: u32,
            frame_offset: u32,
            value_data: *const c_void,
            value_data_size: u32,
        ) where
            F: FnMut(usize, EndpointHandle, EndpointTypeIndex, &[u8]),
        {
            let _result = std::panic::catch_unwind(|| {
                let callback: *mut F = context.cast();
                let callback: &mut F = unsafe { &mut *callback };

                let data = unsafe {
                    std::slice::from_raw_parts(value_data.cast(), value_data_size as usize)
                };
                (*callback)(
                    frame_offset as usize,
                    endpoint.into(),
                    (type_index as usize).into(),
                    data,
                );
            });
        }

        let callback = std::ptr::addr_of_mut!(callback).cast();

        unsafe {
            (self.vtable().iterate_output_events)(
                self.ptr,
                endpoint.into(),
                callback,
                trampoline::<F>,
            )
        };
    }

    pub fn get_xruns(&self) -> usize {
        unsafe { (self.vtable().get_xruns)(self.ptr) as usize }
    }

    pub fn get_max_block_size(&self) -> u32 {
        unsafe { (self.vtable().get_max_block_size)(self.ptr) }
    }

    pub fn get_latency(&self) -> f64 {
        unsafe { (self.vtable().get_latency)(self.ptr) }
    }

    pub fn get_string_for_handle(&self, handle: u32) -> Option<&str> {
        let mut length: isize = 0;
        let ptr = unsafe { (self.vtable().get_string_for_handle)(self.ptr, handle, &mut length) };

        if ptr.is_null() {
            return None;
        }

        let slice = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), length as usize) };

        std::str::from_utf8(slice).ok()
    }
}

impl Drop for PerformerPtr {
    fn drop(&mut self) {
        unsafe { (self.vtable().release)(self.ptr) };
    }
}

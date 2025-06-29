use {
    crate::{
        endpoint::{EndpointDirection, EndpointHandle, EndpointInfo},
        performer::{Endpoint, EndpointError, EndpointType, Performer},
        value::types::{IsScalar, Type},
    },
    std::marker::PhantomData,
};

/// An input stream.
#[derive(Debug, Copy, Clone)]
pub struct InputStream<T>
where
    T: StreamType,
{
    handle: EndpointHandle,
    _marker: PhantomData<T>,
}

/// An output stream.
#[derive(Debug, Copy, Clone)]
pub struct OutputStream<T>
where
    T: StreamType,
{
    handle: EndpointHandle,
    _marker: PhantomData<T>,
}

impl<T> EndpointType for InputStream<T>
where
    T: StreamType,
{
    fn make(
        handle: EndpointHandle,
        endpoint: EndpointInfo,
    ) -> Result<Endpoint<Self>, EndpointError> {
        validate_stream_endpoint::<T>(&endpoint, EndpointDirection::Input)?;

        Ok(Endpoint(Self {
            handle,
            _marker: PhantomData,
        }))
    }

    fn handle(&self) -> EndpointHandle {
        self.handle
    }
}

impl<T> EndpointType for OutputStream<T>
where
    T: StreamType,
{
    fn make(
        handle: EndpointHandle,
        endpoint: EndpointInfo,
    ) -> Result<Endpoint<Self>, EndpointError> {
        validate_stream_endpoint::<T>(&endpoint, EndpointDirection::Output)?;

        Ok(Endpoint(Self {
            handle,
            _marker: PhantomData,
        }))
    }

    fn handle(&self) -> EndpointHandle {
        self.handle
    }
}

fn validate_stream_endpoint<T>(
    endpoint: &EndpointInfo,
    expected_direction: EndpointDirection,
) -> Result<(), EndpointError>
where
    T: StreamType,
{
    if endpoint.direction() != expected_direction {
        return Err(EndpointError::DirectionMismatch);
    }

    let stream = endpoint
        .as_stream()
        .ok_or(EndpointError::EndpointTypeMismatch)?;

    let (stream_type, stream_extent) = match stream.ty() {
        Type::Array(array) => (array.elem_ty(), array.len()),
        ty => (ty, 1),
    };

    if !stream_type.is::<T::Element>() {
        return Err(EndpointError::DataTypeMismatch);
    }

    if stream_extent != T::EXTENT {
        return Err(EndpointError::DataTypeMismatch);
    }

    Ok(())
}

pub fn write_stream<T>(
    performer: &Performer,
    Endpoint(endpoint): Endpoint<InputStream<T>>,
    buffer: &[T],
) where
    T: StreamType,
{
    unsafe { performer.ptr.set_input_frames(endpoint.handle, buffer) }
}

pub fn read_stream<T>(
    performer: &Performer,
    Endpoint(endpoint): Endpoint<OutputStream<T>>,
    buffer: &mut [T],
) where
    T: StreamType,
{
    unsafe {
        performer.ptr.copy_output_frames(endpoint.handle, buffer);
    }
}

pub trait StreamType: Copy + sealed::Sealed {
    type Element: IsScalar + 'static;
    const EXTENT: usize;
}

mod sealed {
    pub trait Sealed {}

    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl<T, const N: usize> Sealed for [T; N] where T: Sealed {}
}

impl StreamType for i32 {
    type Element = Self;
    const EXTENT: usize = 1;
}

impl StreamType for i64 {
    type Element = Self;
    const EXTENT: usize = 1;
}

impl StreamType for f32 {
    type Element = Self;
    const EXTENT: usize = 1;
}

impl StreamType for f64 {
    type Element = Self;
    const EXTENT: usize = 1;
}

impl<T, const EXTENT: usize> StreamType for [T; EXTENT]
where
    T: StreamType,
{
    type Element = T::Element;

    const EXTENT: usize = EXTENT;
}

use {
    crate::{
        endpoint::{EndpointDirection, EndpointHandle, EndpointInfo},
        performer::{
            endpoints::Endpoint, EndpointError, EndpointType, Performer, __seal_endpoint_type,
        },
        value::{Value, ValueRef},
    },
    sealed::sealed,
    std::{any::TypeId, marker::PhantomData},
};

/// An endpoint for input values.
#[derive(Debug, Copy, Clone)]
pub struct InputValue<T = Value> {
    handle: EndpointHandle,
    _marker: PhantomData<fn(T)>,
}

/// An endpoint for output values.
#[derive(Debug, Copy, Clone)]
pub struct OutputValue<T = Value> {
    handle: EndpointHandle,
    _marker: PhantomData<T>,
}

#[sealed]
impl<T> EndpointType for InputValue<T>
where
    T: 'static,
{
    fn make(
        handle: EndpointHandle,
        endpoint: EndpointInfo,
    ) -> Result<Endpoint<Self>, EndpointError> {
        validate_value_endpoint::<T>(&endpoint, EndpointDirection::Input)?;

        Ok(Endpoint(InputValue {
            handle,
            _marker: PhantomData,
        }))
    }

    fn handle(&self) -> EndpointHandle {
        self.handle
    }
}

#[sealed]
impl<T> EndpointType for OutputValue<T>
where
    T: 'static,
{
    fn make(
        handle: EndpointHandle,
        endpoint: EndpointInfo,
    ) -> Result<Endpoint<Self>, EndpointError> {
        validate_value_endpoint::<T>(&endpoint, EndpointDirection::Output)?;

        Ok(Endpoint(OutputValue {
            handle,
            _marker: PhantomData,
        }))
    }

    fn handle(&self) -> EndpointHandle {
        self.handle
    }
}

fn validate_value_endpoint<T>(
    endpoint: &EndpointInfo,
    expected_direction: EndpointDirection,
) -> Result<(), EndpointError>
where
    T: 'static,
{
    if endpoint.direction() != expected_direction {
        return Err(EndpointError::DirectionMismatch);
    }

    let endpoint = endpoint
        .as_value()
        .ok_or(EndpointError::EndpointTypeMismatch)?;

    let user_type = TypeId::of::<T>();
    if user_type != TypeId::of::<Value>() {
        if let Some(endpoint_type) = endpoint.ty().type_id() {
            if user_type != endpoint_type {
                return Err(EndpointError::DataTypeMismatch);
            }
        } else {
            return Err(EndpointError::DataTypeMismatch);
        }
    }

    Ok(())
}

#[doc(hidden)]
pub trait SetInputValue: Sized {
    type Output;

    fn set_input_value(
        performer: &mut Performer,
        endpoint: Endpoint<InputValue<Self>>,
        value: Self,
    ) -> Self::Output;
}

macro_rules! set_input_value_for {
    ($ty:ty) => {
        impl SetInputValue for $ty {
            type Output = ();

            fn set_input_value(
                performer: &mut Performer,
                Endpoint(endpoint): Endpoint<InputValue<Self>>,
                value: Self,
            ) -> Self::Output {
                unsafe {
                    performer
                        .ptr
                        .set_input_value(endpoint.handle, value.to_ne_bytes().as_ptr(), 0);
                }
            }
        }
    };
}

set_input_value_for! {i32}
set_input_value_for! {i64}
set_input_value_for! {f32}
set_input_value_for! {f64}

impl SetInputValue for bool {
    type Output = ();

    fn set_input_value(
        performer: &mut Performer,
        Endpoint(endpoint): Endpoint<InputValue<Self>>,
        value: Self,
    ) -> Self::Output {
        let value: i32 = if value { 1 } else { 0 };
        unsafe {
            performer
                .ptr
                .set_input_value(endpoint.handle, value.to_ne_bytes().as_ptr(), 0);
        }
    }
}

impl SetInputValue for Value {
    type Output = Result<(), EndpointError>;

    fn set_input_value(
        performer: &mut Performer,
        Endpoint(endpoint): Endpoint<InputValue<Self>>,
        value: Self,
    ) -> Self::Output {
        let ty = performer
            .endpoints
            .get(&endpoint.handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?
            .as_value()
            .ok_or(EndpointError::EndpointTypeMismatch)?
            .ty();

        if ty.as_ref() != value.ty() {
            return Err(EndpointError::DataTypeMismatch);
        }

        value.with_bytes(|bytes| unsafe {
            performer
                .ptr
                .set_input_value(endpoint.handle, bytes.as_ptr(), 0);
        });

        Ok(())
    }
}

#[doc(hidden)]
pub trait GetOutputValue: Sized {
    type Output<'a>;

    fn get_output_value(
        performer: &mut Performer,
        endpoint: Endpoint<OutputValue<Self>>,
    ) -> Self::Output<'_>;
}

macro_rules! get_output_value_for {
    ($ty:ty) => {
        impl GetOutputValue for $ty {
            type Output<'a> = Self;

            fn get_output_value(
                performer: &mut Performer,
                Endpoint(endpoint): Endpoint<OutputValue<Self>>,
            ) -> Self::Output<'_> {
                let mut buffer = [0u8; std::mem::size_of::<Self>()];
                performer
                    .ptr
                    .copy_output_value(endpoint.handle, &mut buffer);
                Self::from_ne_bytes(buffer)
            }
        }
    };
}

get_output_value_for! {i32}
get_output_value_for! {i64}
get_output_value_for! {f32}
get_output_value_for! {f64}

impl GetOutputValue for bool {
    type Output<'a> = Self;

    fn get_output_value(
        performer: &mut Performer,
        Endpoint(endpoint): Endpoint<OutputValue<Self>>,
    ) -> Self::Output<'_> {
        let mut buffer = [0u8; size_of::<u32>()];
        performer
            .ptr
            .copy_output_value(endpoint.handle, &mut buffer);
        u32::from_ne_bytes(buffer) != 0
    }
}

impl GetOutputValue for Value {
    type Output<'a> = Result<ValueRef<'a>, ()>;

    fn get_output_value(
        performer: &mut Performer,
        Endpoint(endpoint): Endpoint<OutputValue<Self>>,
    ) -> Self::Output<'_> {
        let Performer { ptr, buffer, .. } = performer;

        let ty = performer
            .endpoints
            .get(&endpoint.handle)
            .and_then(|endpoint| endpoint.as_value())
            .map(|value_endpoint| value_endpoint.ty().as_ref())
            .expect("failed to determine endpoint type");

        ptr.copy_output_value(endpoint.handle, buffer);

        Ok(ValueRef::new_from_slice(ty, &buffer[..ty.size()]))
    }
}

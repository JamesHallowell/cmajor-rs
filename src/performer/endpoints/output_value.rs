use {
    crate::{
        endpoint::{EndpointDirection, EndpointHandle},
        ffi::PerformerPtr,
        performer::{
            atomic::{AtomicF32, AtomicF64},
            endpoints::Endpoint,
            EndpointError, EndpointHandler, Performer, PerformerEndpoint,
            __seal_performer_endpoint,
        },
        value::{
            types::{Primitive, Type},
            Value, ValueRef,
        },
    },
    real_time::writer::realtime_writer,
    sealed::sealed,
    std::{
        any::TypeId,
        marker::PhantomData,
        sync::{
            atomic::{AtomicI32, AtomicI64, Ordering},
            Arc,
        },
    },
};

/// An endpoint for output values.
pub struct OutputValue<T = Value> {
    reader: Reader,
    _marker: PhantomData<fn() -> T>,
}

type Reader = Box<dyn FnMut() -> Value + Send>;

impl Endpoint<OutputValue<bool>> {
    /// Read the value of the endpoint.
    pub fn get(&mut self) -> bool {
        let value: Result<i32, ()> = (self.inner.reader)().as_ref().try_into();
        debug_assert!(value.is_ok());
        value.unwrap_or_default() != 0
    }
}

impl Endpoint<OutputValue<i32>> {
    /// Read the value of the endpoint.
    pub fn get(&mut self) -> i32 {
        let value = (self.inner.reader)().as_ref().try_into();
        debug_assert!(value.is_ok());
        value.unwrap_or_default()
    }
}

impl Endpoint<OutputValue<i64>> {
    /// Read the value of the endpoint.
    pub fn get(&mut self) -> i64 {
        let value = (self.inner.reader)().as_ref().try_into();
        debug_assert!(value.is_ok());
        value.unwrap_or_default()
    }
}

impl Endpoint<OutputValue<f32>> {
    /// Read the value of the endpoint.
    pub fn get(&mut self) -> f32 {
        let value = (self.inner.reader)().as_ref().try_into();
        debug_assert!(value.is_ok());
        value.unwrap_or_default()
    }
}

impl Endpoint<OutputValue<f64>> {
    /// Read the value of the endpoint.
    pub fn get(&mut self) -> f64 {
        let value = (self.inner.reader)().as_ref().try_into();
        debug_assert!(value.is_ok());
        value.unwrap_or_default()
    }
}

impl Endpoint<OutputValue<Value>> {
    /// Read the value of the endpoint.
    pub fn get(&mut self) -> Value {
        (self.inner.reader)()
    }
}

#[sealed]
impl<T> PerformerEndpoint for OutputValue<T>
where
    T: 'static,
{
    fn make<Streams>(
        id: &str,
        performer: &mut Performer<Streams>,
    ) -> Result<Endpoint<Self>, EndpointError> {
        let (handle, endpoint) = performer
            .endpoints
            .get_by_id(id)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.direction() != EndpointDirection::Output {
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

        let (reader, handler) = make_endpoint_connection(handle, endpoint.ty());

        performer.outputs.push(handler);

        Ok(Endpoint {
            inner: OutputValue {
                reader: Box::new(reader),
                _marker: PhantomData,
            },
        })
    }
}

macro_rules! primitive_impl {
    ($handle:ident, $ty:ty, $atomic:ty) => {
        let reader = Arc::new(<$atomic>::default());
        let writer = Arc::clone(&reader);

        return (
            Box::new(move || Value::from(reader.load(Ordering::Relaxed))),
            Box::new(move |performer: &mut PerformerPtr| {
                let mut buffer = [0; std::mem::size_of::<$ty>()];
                performer.copy_output_value($handle, &mut buffer);
                writer.store(<$ty>::from_ne_bytes(buffer), Ordering::Relaxed);
            }),
        )
    };
}

fn make_endpoint_connection(handle: EndpointHandle, ty: &Type) -> (Reader, EndpointHandler) {
    match ty {
        Type::Primitive(Primitive::Bool) => {
            primitive_impl!(handle, i32, AtomicI32);
        }
        Type::Primitive(Primitive::Int32) => {
            primitive_impl!(handle, i32, AtomicI32);
        }
        Type::Primitive(Primitive::Int64) => {
            primitive_impl!(handle, i64, AtomicI64);
        }
        Type::Primitive(Primitive::Float32) => {
            primitive_impl!(handle, f32, AtomicF32);
        }
        Type::Primitive(Primitive::Float64) => {
            primitive_impl!(handle, f64, AtomicF64);
        }
        _ => {
            let (mut reader, mut writer) = realtime_writer(None);

            let ty = ty.to_owned();
            let mut buffer = vec![0; ty.size()];
            return (
                Box::new(move || reader.get().unwrap_or(Value::Void)),
                Box::new(move |performer| {
                    performer.copy_output_value(handle, &mut buffer);
                    writer.set(Some(
                        ValueRef::new_from_slice(ty.as_ref(), &buffer).to_owned(),
                    ));
                }),
            );
        }
    }
}

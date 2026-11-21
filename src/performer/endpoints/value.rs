use {
    crate::{
        endpoint::{EndpointDirection, EndpointInfo, ValueEndpoint},
        performer::{
            endpoints::{sealed::Sealed, EndpointKind},
            Endpoint, EndpointError, Performer, TypeMismatch,
        },
        value::{types::Type, StaticType, Value, ValueRef},
    },
    std::{fmt, marker::PhantomData},
};

/// An endpoint for input values of type `T`.
pub struct InputValue<T = Value>(PhantomData<fn(T)>);

/// An endpoint for output values of type `T`.
pub struct OutputValue<T = Value>(PhantomData<fn() -> T>);

impl<T> Sealed for InputValue<T> {}
impl<T> Sealed for OutputValue<T> {}

impl<T: ValueType> EndpointKind for InputValue<T> {
    type Binding = T::Binding;

    fn bind(info: &EndpointInfo) -> Result<T::Binding, EndpointError> {
        T::bind(value_endpoint(info, EndpointDirection::Input)?)
    }
}

impl<T: ValueType> EndpointKind for OutputValue<T> {
    type Binding = T::Binding;

    fn bind(info: &EndpointInfo) -> Result<T::Binding, EndpointError> {
        T::bind(value_endpoint(info, EndpointDirection::Output)?)
    }
}

fn value_endpoint(
    info: &EndpointInfo,
    expected_direction: EndpointDirection,
) -> Result<&ValueEndpoint, EndpointError> {
    if info.direction() != expected_direction {
        return Err(EndpointError::DirectionMismatch);
    }

    info.as_value().ok_or(EndpointError::EndpointTypeMismatch)
}

#[doc(hidden)]
pub trait ValueType: Sized {
    type Binding: Clone + fmt::Debug;
    type SetResult;
    type Value<'a>
    where
        Self: 'a;

    fn bind(endpoint: &ValueEndpoint) -> Result<Self::Binding, EndpointError>;

    fn set(
        performer: &mut Performer,
        endpoint: &Endpoint<InputValue<Self>>,
        value: Self,
    ) -> Self::SetResult;

    fn get<'a>(
        performer: &'a mut Performer,
        endpoint: &'a Endpoint<OutputValue<Self>>,
    ) -> Self::Value<'a>;
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct BoundDynamicValueType {
    ty: Type,
}

impl<T: StaticType> ValueType for T {
    type Binding = ();
    type SetResult = ();
    type Value<'a>
        = T
    where
        T: 'a;

    fn bind(endpoint: &ValueEndpoint) -> Result<(), EndpointError> {
        if !endpoint.ty().is::<T>() {
            return Err(EndpointError::DataTypeMismatch);
        }

        Ok(())
    }

    fn set(performer: &mut Performer, endpoint: &Endpoint<InputValue<T>>, value: T) {
        let handle = endpoint.handle_on(performer);
        let Performer { ptr, buffer, .. } = performer;

        let bytes = value.write(buffer);
        unsafe { ptr.set_input_value(handle, bytes.as_ptr(), 0) };
    }

    fn get(performer: &mut Performer, endpoint: &Endpoint<OutputValue<T>>) -> T {
        let handle = endpoint.handle_on(performer);
        let Performer { ptr, buffer, .. } = performer;

        ptr.copy_output_value(handle, buffer);
        T::read(buffer).0
    }
}

impl ValueType for Value {
    type Binding = BoundDynamicValueType;
    type SetResult = Result<(), TypeMismatch>;
    type Value<'a> = ValueRef<'a>;

    fn bind(endpoint: &ValueEndpoint) -> Result<BoundDynamicValueType, EndpointError> {
        Ok(BoundDynamicValueType {
            ty: endpoint.ty().clone(),
        })
    }

    fn set(
        performer: &mut Performer,
        endpoint: &Endpoint<InputValue<Value>>,
        value: Value,
    ) -> Result<(), TypeMismatch> {
        let handle = endpoint.handle_on(performer);
        if endpoint.binding().ty.as_ref() != value.ty() {
            return Err(TypeMismatch);
        }

        value.with_bytes(|bytes| unsafe {
            performer.ptr.set_input_value(handle, bytes.as_ptr(), 0);
        });

        Ok(())
    }

    fn get<'a>(
        performer: &'a mut Performer,
        endpoint: &'a Endpoint<OutputValue<Value>>,
    ) -> ValueRef<'a> {
        let handle = endpoint.handle_on(performer);
        let Performer { ptr, buffer, .. } = performer;
        let ty = endpoint.binding().ty.as_ref();

        ptr.copy_output_value(handle, buffer);
        ValueRef::new_from_slice(ty, buffer)
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{performer::endpoints::test, value::types::Array},
    };

    fn value_endpoint(direction: EndpointDirection, ty: impl Into<Type>) -> EndpointInfo {
        ValueEndpoint::new(test::id("v"), direction, ty.into(), Default::default()).into()
    }

    #[test]
    fn a_typed_input_value_binds_to_an_endpoint_of_its_type() {
        let info = value_endpoint(EndpointDirection::Input, Type::Float32);

        assert!(InputValue::<f32>::bind(&info).is_ok());
    }

    #[test]
    fn a_typed_input_value_rejects_an_endpoint_of_another_type() {
        let info = value_endpoint(EndpointDirection::Input, Type::Int32);

        assert!(matches!(
            InputValue::<f32>::bind(&info),
            Err(EndpointError::DataTypeMismatch)
        ));
    }

    #[test]
    fn an_array_value_binds_to_an_array_endpoint_of_the_same_shape() {
        let info = value_endpoint(EndpointDirection::Output, Array::new(Type::Int32, 4));

        assert!(OutputValue::<[i32; 4]>::bind(&info).is_ok());
        assert!(matches!(
            OutputValue::<[i32; 3]>::bind(&info),
            Err(EndpointError::DataTypeMismatch)
        ));
    }

    #[test]
    fn a_value_placeholder_binds_to_any_value_endpoint_type() {
        let info = value_endpoint(EndpointDirection::Input, Type::String);

        let binding = InputValue::<Value>::bind(&info).unwrap();

        assert_eq!(binding.ty, Type::String);
    }

    #[test]
    fn an_output_value_cannot_bind_to_an_input_endpoint() {
        let info = value_endpoint(EndpointDirection::Input, Type::Int32);

        assert!(matches!(
            OutputValue::<i32>::bind(&info),
            Err(EndpointError::DirectionMismatch)
        ));
    }
}

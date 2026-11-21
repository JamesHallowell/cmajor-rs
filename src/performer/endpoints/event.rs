use {
    crate::{
        endpoint::{EndpointDirection, EndpointInfo, EndpointTypeIndex, EventEndpoint},
        performer::{
            endpoints::{sealed::Sealed, EndpointKind},
            Endpoint, EndpointError, Performer, TypeMismatch,
        },
        value::{types::Type, StaticType, Value, ValueRef},
    },
    std::{fmt, marker::PhantomData, sync::Arc},
};

/// An endpoint for input events of type `T`.
pub struct InputEvent<T = Value>(PhantomData<fn(T)>);

/// An endpoint for output events of type `T`.
pub struct OutputEvent<T = Value>(PhantomData<fn() -> T>);

impl<T> Sealed for InputEvent<T> {}
impl<T> Sealed for OutputEvent<T> {}

impl<T: EventType> EndpointKind for InputEvent<T> {
    type Binding = T::Binding;

    fn bind(info: &EndpointInfo) -> Result<T::Binding, EndpointError> {
        T::bind(event_endpoint(info, EndpointDirection::Input)?)
    }
}

impl<T: EventType> EndpointKind for OutputEvent<T> {
    type Binding = T::Binding;

    fn bind(info: &EndpointInfo) -> Result<T::Binding, EndpointError> {
        T::bind(event_endpoint(info, EndpointDirection::Output)?)
    }
}

fn event_endpoint(
    info: &EndpointInfo,
    expected_direction: EndpointDirection,
) -> Result<&EventEndpoint, EndpointError> {
    if info.direction() != expected_direction {
        return Err(EndpointError::DirectionMismatch);
    }

    info.as_event().ok_or(EndpointError::EndpointTypeMismatch)
}

/// The type of an event endpoint.
#[doc(hidden)]
pub trait EventType: Sized {
    type Binding: Clone + fmt::Debug;
    type Event<'a>;
    type PostResult;

    fn bind(endpoint: &EventEndpoint) -> Result<Self::Binding, EndpointError>;

    fn post(
        performer: &mut Performer,
        endpoint: &Endpoint<InputEvent<Self>>,
        event: Self::Event<'_>,
    ) -> Self::PostResult;

    fn fetch(
        performer: &Performer,
        endpoint: &Endpoint<OutputEvent<Self>>,
        callback: impl FnMut(usize, Self::Event<'_>),
    );
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct BoundStaticEventType {
    index: EndpointTypeIndex,
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct BoundDynamicEventType {
    types: Arc<[Type]>,
}

impl<T: StaticType> EventType for T {
    type Binding = BoundStaticEventType;
    type Event<'a> = T;
    type PostResult = ();

    fn bind(endpoint: &EventEndpoint) -> Result<BoundStaticEventType, EndpointError> {
        let index = endpoint
            .type_index(T::ty().as_ref())
            .ok_or(EndpointError::DataTypeMismatch)?;

        Ok(BoundStaticEventType { index })
    }

    fn post(performer: &mut Performer, endpoint: &Endpoint<InputEvent<T>>, event: T) {
        let handle = endpoint.handle_on(performer);
        let BoundStaticEventType { index } = *endpoint.binding();
        let Performer { ptr, buffer, .. } = performer;

        let bytes = event.write(buffer);

        ptr.add_input_event(handle, index, bytes);
    }

    fn fetch(
        performer: &Performer,
        endpoint: &Endpoint<OutputEvent<T>>,
        mut callback: impl FnMut(usize, T),
    ) {
        performer.ptr.iterate_output_events(
            endpoint.handle_on(performer),
            |frame_offset, _, type_index, data| {
                let &BoundStaticEventType { index } = endpoint.binding();
                if type_index == index {
                    callback(frame_offset, T::read(data).0);
                }
            },
        );
    }
}

impl EventType for Value {
    type Binding = BoundDynamicEventType;
    type Event<'a> = ValueRef<'a>;
    type PostResult = Result<(), TypeMismatch>;

    fn bind(endpoint: &EventEndpoint) -> Result<BoundDynamicEventType, EndpointError> {
        Ok(BoundDynamicEventType {
            types: endpoint.types().into(),
        })
    }

    fn post(
        performer: &mut Performer,
        endpoint: &Endpoint<InputEvent<Value>>,
        event: ValueRef<'_>,
    ) -> Result<(), TypeMismatch> {
        let handle = endpoint.handle_on(performer);
        let type_index = endpoint
            .binding()
            .types
            .iter()
            .position(|ty| ty.as_ref() == event.ty())
            .ok_or(TypeMismatch)?;

        event.with_bytes(|bytes| {
            performer
                .ptr
                .add_input_event(handle, type_index.into(), bytes);
        });

        Ok(())
    }

    fn fetch(
        performer: &Performer,
        endpoint: &Endpoint<OutputEvent<Value>>,
        mut callback: impl FnMut(usize, ValueRef<'_>),
    ) {
        performer.ptr.iterate_output_events(
            endpoint.handle_on(performer),
            |frame_offset, _, type_index, data| {
                let ty = endpoint.binding().types.get(usize::from(type_index));
                debug_assert!(ty.is_some(), "Invalid type index from Cmajor");

                if let Some(ty) = ty {
                    callback(frame_offset, ValueRef::new_from_slice(ty.as_ref(), data));
                }
            },
        );
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{endpoint::ValueEndpoint, performer::endpoints::test},
    };

    fn event_endpoint(direction: EndpointDirection, types: Vec<Type>) -> EndpointInfo {
        EventEndpoint::new(test::id("e"), direction, types, Default::default()).into()
    }

    #[test]
    fn a_typed_input_event_binds_to_the_position_of_its_type() {
        let info = event_endpoint(EndpointDirection::Input, vec![Type::Int32, Type::Bool]);

        assert_eq!(
            InputEvent::<i32>::bind(&info).unwrap().index,
            EndpointTypeIndex::from(0)
        );
        assert_eq!(
            InputEvent::<bool>::bind(&info).unwrap().index,
            EndpointTypeIndex::from(1)
        );
    }

    #[test]
    fn a_typed_input_event_rejects_a_type_the_endpoint_does_not_carry() {
        let info = event_endpoint(EndpointDirection::Input, vec![Type::Int32, Type::Bool]);

        assert!(matches!(
            InputEvent::<f32>::bind(&info),
            Err(EndpointError::DataTypeMismatch)
        ));
    }

    #[test]
    fn an_input_event_cannot_bind_to_an_output_endpoint() {
        let info = event_endpoint(EndpointDirection::Output, vec![Type::Int32]);

        assert!(matches!(
            InputEvent::<i32>::bind(&info),
            Err(EndpointError::DirectionMismatch)
        ));
    }

    #[test]
    fn an_input_event_cannot_bind_to_a_value_endpoint() {
        let info = ValueEndpoint::new(
            test::id("v"),
            EndpointDirection::Input,
            Type::Int32,
            Default::default(),
        )
        .into();

        assert!(matches!(
            InputEvent::<i32>::bind(&info),
            Err(EndpointError::EndpointTypeMismatch)
        ));
    }

    #[test]
    fn an_event_placeholder_binds_to_every_type_of_the_endpoint() {
        let info = event_endpoint(EndpointDirection::Output, vec![Type::Int32, Type::Bool]);

        let binding = OutputEvent::<Value>::bind(&info).unwrap();

        assert_eq!(*binding.types, [Type::Int32, Type::Bool]);
    }
}

use {
    crate::{
        engine::{EndpointHandle, EndpointType, Endpoints},
        performer::spsc,
        value::Value,
    },
    std::sync::Arc,
};

pub struct EndpointHandles {
    pub(super) endpoints: Arc<Endpoints>,
    pub(super) endpoint_tx: spsc::EndpointSender,
}

#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    #[error("no such endpoint")]
    EndpointDoesNotExist,

    #[error("type mismatch")]
    EndpointTypeMismatch,

    #[error("data type mismatch")]
    DataTypeMismatch,

    #[error("failed to send value")]
    FailedToSendValue,
}

impl EndpointHandles {
    pub fn get_input(&self, id: impl AsRef<str>) -> Option<EndpointHandle> {
        self.endpoints.get_input_by_id(id).map(|(handle, _)| handle)
    }

    pub fn write_value(
        &mut self,
        handle: EndpointHandle,
        value: impl Into<Value>,
    ) -> Result<(), EndpointError> {
        let endpoint = self
            .endpoints
            .get_input(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Value {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        let value = value.into();

        if !endpoint.value_type().contains(value.ty()) {
            return Err(EndpointError::DataTypeMismatch);
        }

        self.endpoint_tx
            .send_value(handle, value.data())
            .map_err(|_| EndpointError::FailedToSendValue)
    }

    pub fn post_event(
        &mut self,
        handle: EndpointHandle,
        value: impl Into<Value>,
    ) -> Result<(), EndpointError> {
        let endpoint = self
            .endpoints
            .get_input(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Event {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        let value = value.into();

        let index = endpoint
            .value_type()
            .iter()
            .enumerate()
            .find_map(|(index, ty)| (ty == value.ty()).then_some(index))
            .ok_or(EndpointError::DataTypeMismatch)?;

        self.endpoint_tx
            .send_event(handle, index as u32, value.data())
            .map_err(|_| EndpointError::FailedToSendValue)
    }
}

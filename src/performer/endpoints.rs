use {
    crate::{
        engine::{EndpointHandle, EndpointType, Endpoints},
        performer::{spsc, spsc::EndpointMessage},
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

    #[error("failed to send message to performer")]
    FailedToSendMessageToPerformer,
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

        let message = EndpointMessage::Value {
            handle,
            data: value.data(),
            num_frames_to_reach_value: 0,
        };

        self.endpoint_tx
            .send(message)
            .map_err(|_| EndpointError::FailedToSendMessageToPerformer)
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

        let type_index = endpoint
            .index_of_value_type(value.ty())
            .ok_or(EndpointError::DataTypeMismatch)?;

        let message = EndpointMessage::Event {
            handle,
            type_index,
            data: value.data(),
        };

        self.endpoint_tx
            .send(message)
            .map_err(|_| EndpointError::FailedToSendMessageToPerformer)
    }
}

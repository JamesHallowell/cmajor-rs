use {
    crate::endpoint::buffer::Producer,
    serde::Serialize,
    std::{borrow::Borrow, marker::PhantomData},
};

mod buffer;
mod details;

pub struct Endpoints {
    buffer: Producer,
}

pub struct Endpoint<'a, EndpointType, DataType> {
    endpoints: &'a mut Endpoints,
    _marker: PhantomData<(EndpointType, DataType)>,
}

pub struct Value;

impl Endpoints {
    pub fn get_value_endpoint<Type>(
        &mut self,
        _name: impl AsRef<str>,
    ) -> Endpoint<'_, Value, Type> {
        Endpoint {
            endpoints: self,
            _marker: PhantomData,
        }
    }
}

impl<DataType> Endpoint<'_, Value, DataType> {
    pub fn post(&mut self, value: impl Borrow<DataType>)
    where
        DataType: Serialize,
    {
        self.endpoints.buffer.write(value.borrow()).unwrap();
    }
}

mod test {
    use {super::*, crate::endpoint::buffer::buffer};

    #[test]
    fn example() {
        let (producer, consumer) = buffer(1024);

        let mut endpoints = Endpoints { buffer: producer };

        endpoints.get_value_endpoint("other").post(5);
        endpoints.get_value_endpoint("someother").post(5);
    }
}

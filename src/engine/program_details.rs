use {
    crate::{
        endpoint::{
            EndpointDirection, EndpointId, EndpointType, EventEndpoint, StreamEndpoint,
            ValueEndpoint,
        },
        ffi::types::{TypeDescription, TypeDescriptionError},
        value::types::Type,
    },
    serde::{
        de::{value::MapAccessDeserializer, Visitor},
        Deserialize, Deserializer,
    },
    serde_json::{Map as JsonMap, Value as JsonValue},
    std::{fmt::Formatter, iter::repeat},
};

/// Details about a Cmajor program.
#[derive(Debug, Deserialize)]
pub struct ProgramDetails {
    inputs: Vec<EndpointDetails>,
    outputs: Vec<EndpointDetails>,
    #[serde(flatten)]
    _extra: JsonMap<String, JsonValue>,
}

impl ProgramDetails {
    /// Returns an iterator over all the endpoints in the program.
    pub fn endpoints(&self) -> impl Iterator<Item = EndpointType> + '_ {
        let inputs = self.inputs.iter().zip(repeat(EndpointDirection::Input));
        let outputs = self.outputs.iter().zip(repeat(EndpointDirection::Output));

        inputs.chain(outputs).filter_map(|(details, direction)| {
            match try_make_endpoint(details, direction) {
                Ok(endpoint) => Some(endpoint),
                Err(err) => {
                    eprintln!("failed to parse endpoint: {:?}", err);
                    None
                }
            }
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct EndpointDetails {
    #[serde(rename = "endpointID")]
    id: EndpointId,

    #[serde(rename = "endpointType")]
    endpoint_type: EndpointVariant,

    #[serde(
        rename = "dataType",
        alias = "dataTypes",
        deserialize_with = "deserialize_data_type"
    )]
    value_type: Vec<Type>,

    #[serde(rename = "annotation")]
    annotation: Option<JsonMap<String, JsonValue>>,

    #[serde(flatten)]
    _extra: JsonMap<String, JsonValue>,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq)]
enum EndpointVariant {
    #[serde(rename = "stream")]
    Stream,

    #[serde(rename = "event")]
    Event,

    #[serde(rename = "value")]
    Value,
}

fn try_make_endpoint(
    EndpointDetails {
        id,
        endpoint_type,
        value_type,
        annotation,
        ..
    }: &EndpointDetails,
    direction: EndpointDirection,
) -> Result<EndpointType, TypeDescriptionError> {
    let annotation = annotation.clone().unwrap_or_default().into();

    Ok(match endpoint_type {
        EndpointVariant::Stream => {
            if value_type.len() != 1 {
                return Err(TypeDescriptionError::UnexpectedNumberOfTypes);
            }

            StreamEndpoint::new(id.clone(), direction, value_type[0].clone(), annotation).into()
        }
        EndpointVariant::Event => {
            EventEndpoint::new(id.clone(), direction, value_type.clone(), annotation).into()
        }
        EndpointVariant::Value => {
            if value_type.len() != 1 {
                return Err(TypeDescriptionError::UnexpectedNumberOfTypes);
            }

            ValueEndpoint::new(id.clone(), direction, value_type[0].clone(), annotation).into()
        }
    })
}

fn deserialize_data_type<'de, D>(deserializer: D) -> Result<Vec<Type>, D::Error>
where
    D: Deserializer<'de>,
{
    struct DataTypesVisitor;

    impl<'de> Visitor<'de> for DataTypesVisitor {
        type Value = Vec<Type>;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("a data type or a list of data types")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut data_types = Vec::new();
            while let Some(data_type) = seq.next_element::<TypeDescription>()? {
                let data_type = Type::try_from(&data_type).map_err(serde::de::Error::custom)?;
                data_types.push(data_type);
            }

            Ok(data_types)
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let data_type: TypeDescription =
                Deserialize::deserialize(MapAccessDeserializer::new(map))?;

            let data_type = Type::try_from(&data_type).map_err(serde::de::Error::custom)?;

            Ok(vec![data_type])
        }
    }

    deserializer.deserialize_any(DataTypesVisitor)
}

#[cfg(test)]
mod test {
    use {super::*, crate::value::types::Primitive};

    #[test]
    fn parse_an_endpoint_with_a_single_data_type() {
        let json = r#"
            {
                "endpointID": "out",
                "endpointType": "stream",
                "dataType": {
                    "type": "float32"
                }
            }
        "#;

        let details: EndpointDetails = serde_json::from_str(json).unwrap();

        assert_eq!(details.id.as_ref(), "out");
        assert_eq!(details.endpoint_type, EndpointVariant::Stream);
        assert_eq!(
            details.value_type,
            vec![Type::Primitive(Primitive::Float32)]
        );
    }

    #[test]
    fn parse_an_endpoint_with_a_multiple_data_type() {
        let json = r#"
            {
                "endpointID": "out",
                "endpointType": "event",
                "dataTypes": [
                    {
                        "type": "float32"
                    },
                    {
                        "type": "int32"
                    }
                ]
            }
        "#;

        let details: EndpointDetails = serde_json::from_str(json).unwrap();

        assert_eq!(details.id.as_ref(), "out");
        assert_eq!(details.endpoint_type, EndpointVariant::Event);
        assert_eq!(
            details.value_type,
            vec![
                Type::Primitive(Primitive::Float32),
                Type::Primitive(Primitive::Int32)
            ]
        );
    }
}

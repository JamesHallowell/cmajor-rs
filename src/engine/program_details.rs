use {
    crate::{
        endpoint::{
            Endpoint, EndpointDirection, EndpointId, EventEndpoint, StreamEndpoint, ValueEndpoint,
        },
        engine::program_details::ParseEndpointError::UnsupportedType,
        value::types::{Array, Object, Primitive, Type},
    },
    serde::{
        de::{value::MapAccessDeserializer, Visitor},
        Deserialize, Deserializer,
    },
    serde_json::{Map as JsonMap, Value as JsonValue},
    std::{borrow::Borrow, fmt::Formatter, iter::repeat},
};

#[derive(Debug, Deserialize)]
pub struct ProgramDetails {
    inputs: Vec<EndpointDetails>,
    outputs: Vec<EndpointDetails>,
    #[serde(flatten)]
    _extra: JsonMap<String, JsonValue>,
}

impl ProgramDetails {
    pub fn endpoints(&self) -> impl Iterator<Item = Endpoint> + '_ {
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
    endpoint_type: EndpointType,

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
enum EndpointType {
    #[serde(rename = "stream")]
    Stream,

    #[serde(rename = "event")]
    Event,

    #[serde(rename = "value")]
    Value,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq)]
enum ValueType {
    #[serde(rename = "void")]
    Void,

    #[serde(rename = "bool")]
    Bool,

    #[serde(rename = "int32")]
    Int32,

    #[serde(rename = "int64")]
    Int64,

    #[serde(rename = "float32")]
    Float32,

    #[serde(rename = "float64")]
    Float64,

    #[serde(rename = "string")]
    String,

    #[serde(rename = "array")]
    Array,

    #[serde(rename = "object")]
    Object,

    #[serde(rename = "vector")]
    Vector,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct EndpointDataType {
    #[serde(rename = "type")]
    ty: ValueType,

    #[serde(rename = "class")]
    class: Option<String>,

    #[serde(rename = "members")]
    members: Option<JsonMap<String, JsonValue>>,

    #[serde(rename = "element")]
    element: Option<Box<Self>>,

    #[serde(rename = "size")]
    size: Option<usize>,

    #[serde(flatten)]
    _extra: JsonMap<String, JsonValue>,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseEndpointError {
    #[error(transparent)]
    InvalidJson(#[from] serde_json::Error),

    #[error("unsupported type: {0:?}")]
    UnsupportedType(String),

    #[error("struct has no members")]
    StructHasNoMembers,

    #[error("array has no element")]
    ArrayHasNoElement,

    #[error("array has no size")]
    ArrayHasNoSize,

    #[error("endpoint has an unexpected number of types")]
    UnexpectedNumberOfTypes,
}

impl TryFrom<&EndpointDataType> for Type {
    type Error = ParseEndpointError;

    fn try_from(
        EndpointDataType {
            ty,
            members,
            element,
            size,
            ..
        }: &EndpointDataType,
    ) -> Result<Self, Self::Error> {
        match *ty {
            ValueType::Void => Ok(Type::Primitive(Primitive::Void)),
            ValueType::Bool => Ok(Type::Primitive(Primitive::Bool)),
            ValueType::Int32 => Ok(Type::Primitive(Primitive::Int32)),
            ValueType::Int64 => Ok(Type::Primitive(Primitive::Int64)),
            ValueType::Float32 => Ok(Type::Primitive(Primitive::Float32)),
            ValueType::Float64 => Ok(Type::Primitive(Primitive::Float64)),
            ValueType::Object => {
                let mut object = Object::new();
                for (name, value) in members.as_ref().ok_or(Self::Error::StructHasNoMembers)? {
                    let ty: Type = serde_json::from_value::<EndpointDataType>(value.clone())?
                        .borrow()
                        .try_into()?;
                    object.add_field(name, ty);
                }
                Ok(object.into())
            }
            ValueType::Array | ValueType::Vector => {
                let element_ty: Type = element
                    .as_ref()
                    .ok_or(ParseEndpointError::ArrayHasNoElement)?
                    .as_ref()
                    .try_into()?;
                let size = size.ok_or(ParseEndpointError::ArrayHasNoSize)?;

                Ok(Array::new(element_ty, size).into())
            }
            ty => Err(UnsupportedType(format!("{:?}", ty))),
        }
    }
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
) -> Result<Endpoint, ParseEndpointError> {
    let annotation = annotation.clone().unwrap_or_default().into();

    Ok(match endpoint_type {
        EndpointType::Stream => {
            if value_type.len() != 1 {
                return Err(ParseEndpointError::UnexpectedNumberOfTypes);
            }

            StreamEndpoint::new(id.clone(), direction, value_type[0].clone(), annotation).into()
        }
        EndpointType::Event => {
            EventEndpoint::new(id.clone(), direction, value_type.clone(), annotation).into()
        }
        EndpointType::Value => {
            if value_type.len() != 1 {
                return Err(ParseEndpointError::UnexpectedNumberOfTypes);
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
            while let Some(data_type) = seq.next_element::<EndpointDataType>()? {
                let data_type = Type::try_from(&data_type).map_err(serde::de::Error::custom)?;
                data_types.push(data_type);
            }

            Ok(data_types)
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let data_type: EndpointDataType =
                Deserialize::deserialize(MapAccessDeserializer::new(map))?;

            let data_type = Type::try_from(&data_type).map_err(serde::de::Error::custom)?;

            Ok(vec![data_type])
        }
    }

    deserializer.deserialize_any(DataTypesVisitor)
}

#[cfg(test)]
mod test {
    use super::*;

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
        assert_eq!(details.endpoint_type, EndpointType::Stream);
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
        assert_eq!(details.endpoint_type, EndpointType::Event);
        assert_eq!(
            details.value_type,
            vec![
                Type::Primitive(Primitive::Float32),
                Type::Primitive(Primitive::Int32)
            ]
        );
    }
}

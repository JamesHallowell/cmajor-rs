use {
    crate::{Object, Type},
    serde::{
        de::{value::MapAccessDeserializer, Visitor},
        Deserialize, Deserializer,
    },
    serde_json::{Map as JsonMap, Value as JsonValue},
    std::{borrow::Borrow, fmt::Formatter},
};

#[derive(Debug, Deserialize)]
pub struct ProgramDetails {
    pub inputs: Vec<EndpointDetails>,
    pub outputs: Vec<EndpointDetails>,
    #[serde(flatten)]
    _extra: JsonMap<String, JsonValue>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct EndpointId(String);

impl AsRef<str> for EndpointId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for EndpointId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EndpointDetails {
    #[serde(rename = "endpointID")]
    pub id: EndpointId,

    #[serde(rename = "endpointType")]
    pub endpoint_type: EndpointType,

    #[serde(
        rename = "dataType",
        alias = "dataTypes",
        deserialize_with = "deserialize_data_type"
    )]
    pub value_type: Vec<Type>,

    #[serde(flatten)]
    _extra: JsonMap<String, JsonValue>,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq)]
pub enum EndpointType {
    #[serde(rename = "stream")]
    Stream,
    #[serde(rename = "event")]
    Event,
    #[serde(rename = "value")]
    Value,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq)]
pub enum ValueType {
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
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EndpointDataType {
    #[serde(rename = "type")]
    r#type: ValueType,

    #[serde(rename = "class")]
    class: Option<String>,

    #[serde(rename = "members")]
    members: Option<JsonMap<String, JsonValue>>,

    #[serde(flatten)]
    _extra: JsonMap<String, JsonValue>,
}

fn convert_type(value_type: EndpointDataType) -> Type {
    match value_type.r#type {
        ValueType::Void => Type::Void,
        ValueType::Bool => Type::Bool,
        ValueType::Int32 => Type::Int32,
        ValueType::Int64 => Type::Int64,
        ValueType::Float32 => Type::Float32,
        ValueType::Float64 => Type::Float64,
        ValueType::String => Type::String,
        ValueType::Object => {
            let mut object = Object::new();
            for (name, value) in value_type.members.unwrap_or_default() {
                let value: EndpointDataType = serde_json::from_value(value).unwrap();
                let field_type = convert_type(value);
                object.add_field(name, field_type);
            }
            object.into()
        }
        _ => unimplemented!("unsupported data type"),
    }
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
                let data_type = convert_type(data_type);
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

            Ok(vec![convert_type(data_type)])
        }
    }

    deserializer.deserialize_any(DataTypesVisitor)
}

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
        assert_eq!(details.value_type, vec![Type::Float32]);
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
        assert_eq!(details.value_type, vec![Type::Float32, Type::Int32]);
    }
}

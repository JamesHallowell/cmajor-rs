use {
    crate::types::Type,
    serde::{
        de::{value::MapAccessDeserializer, Visitor},
        Deserialize, Deserializer,
    },
    serde_json::{Map as JsonMap, Value as JsonValue},
    std::{borrow::Borrow, fmt::Formatter},
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EndpointDirection {
    Input,
    Output,
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

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EndpointDataType {
    #[serde(rename = "type")]
    r#type: Type,

    #[serde(rename = "class")]
    class: Option<String>,

    #[serde(rename = "members")]
    members: Option<JsonMap<String, JsonValue>>,
}

impl EndpointDataType {
    pub fn data_type(&self) -> Type {
        self.r#type
    }

    pub fn class(&self) -> Option<&str> {
        self.class.as_ref().map(String::as_str)
    }

    pub fn members(&self) -> Option<impl Iterator<Item = (&str, Self)> + '_> {
        self.members.as_ref().map(|members| {
            members.iter().filter_map(|(name, value)| {
                let member = serde_json::from_value(value.clone()).ok()?;
                Some((name.as_str(), member))
            })
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct EndpointId(String);

impl Borrow<str> for EndpointId {
    fn borrow(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EndpointDetails {
    #[serde(rename = "endpointID")]
    id: EndpointId,

    #[serde(rename = "endpointType")]
    r#type: EndpointType,

    #[serde(
        rename = "dataType",
        alias = "dataTypes",
        deserialize_with = "deserialize_data_type"
    )]
    data_type: Vec<EndpointDataType>,
}

fn deserialize_data_type<'de, D>(deserializer: D) -> Result<Vec<EndpointDataType>, D::Error>
where
    D: Deserializer<'de>,
{
    struct DataTypesVisitor;

    impl<'de> Visitor<'de> for DataTypesVisitor {
        type Value = Vec<EndpointDataType>;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("a data type or a list of data types")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut data_types = Vec::new();
            while let Some(data_type) = seq.next_element()? {
                data_types.push(data_type);
            }
            Ok(data_types)
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            Deserialize::deserialize(MapAccessDeserializer::new(map)).map(|a| vec![a])
        }
    }

    deserializer.deserialize_any(DataTypesVisitor)
}

impl EndpointId {
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl EndpointDetails {
    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    pub fn endpoint_type(&self) -> EndpointType {
        self.r#type
    }

    pub fn data_type(&self) -> impl Iterator<Item = EndpointDataType> + '_ {
        self.data_type.iter().cloned()
    }
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

        assert_eq!(details.id().as_str(), "out");
        assert_eq!(details.endpoint_type(), EndpointType::Stream);
        assert_eq!(
            details.data_type().collect::<Vec<_>>(),
            vec![EndpointDataType {
                r#type: Type::Float32,
                class: None,
                members: None,
            }]
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

        assert_eq!(details.id().as_str(), "out");
        assert_eq!(details.endpoint_type(), EndpointType::Event);
        assert_eq!(
            details.data_type().collect::<Vec<_>>(),
            vec![
                EndpointDataType {
                    r#type: Type::Float32,
                    class: None,
                    members: None,
                },
                EndpointDataType {
                    r#type: Type::Int32,
                    class: None,
                    members: None,
                }
            ]
        );
    }
}

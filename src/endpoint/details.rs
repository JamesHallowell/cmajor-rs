use {
    serde::{
        de::{value::MapAccessDeserializer, Visitor},
        Deserialize, Deserializer,
    },
    std::fmt::Formatter,
};

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
pub enum DataType {
    #[serde(rename = "void")]
    Void,

    #[serde(rename = "int32")]
    Int32,

    #[serde(rename = "int64")]
    Int64,

    #[serde(rename = "float32")]
    Float32,

    #[serde(rename = "float64")]
    Float64,

    #[serde(rename = "bool")]
    Bool,

    #[serde(rename = "string")]
    String,

    #[serde(rename = "vector")]
    Vector,

    #[serde(rename = "array")]
    Array,

    #[serde(rename = "object")]
    Object,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq)]
pub struct EndpointDataType {
    #[serde(rename = "type")]
    r#type: DataType,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct EndpointDetails {
    #[serde(rename = "endpointID")]
    id: String,

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

impl EndpointDetails {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn endpoint_type(&self) -> EndpointType {
        self.r#type
    }

    pub fn data_type(&self) -> impl Iterator<Item = EndpointDataType> + '_ {
        self.data_type.iter().copied()
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

        assert_eq!(details.id(), "out");
        assert_eq!(details.endpoint_type(), EndpointType::Stream);
        assert_eq!(
            details.data_type().collect::<Vec<_>>(),
            vec![EndpointDataType {
                r#type: DataType::Float32
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

        assert_eq!(details.id(), "out");
        assert_eq!(details.endpoint_type(), EndpointType::Event);
        assert_eq!(
            details.data_type().collect::<Vec<_>>(),
            vec![
                EndpointDataType {
                    r#type: DataType::Float32
                },
                EndpointDataType {
                    r#type: DataType::Int32
                }
            ]
        );
    }
}

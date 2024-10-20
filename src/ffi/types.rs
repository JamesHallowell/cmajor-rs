use {
    crate::value::types::{Array, Object, Primitive, Type},
    indexmap::IndexMap,
    serde::Deserialize,
    serde_json as json,
};

#[derive(Debug, Copy, Clone, Deserialize)]
enum TypeTag {
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

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TypeDescription {
    #[serde(rename = "type")]
    type_tag: TypeTag,

    #[serde(rename = "class")]
    class: Option<String>,

    #[serde(rename = "members")]
    members: Option<IndexMap<String, Self>>,

    #[serde(rename = "element")]
    element: Option<Box<Self>>,

    #[serde(rename = "size")]
    size: Option<usize>,

    #[serde(flatten)]
    _extra: json::Map<String, json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TypeDescriptionError {
    #[error(transparent)]
    InvalidJson(#[from] json::Error),

    #[error("struct has no class")]
    StructHasNoClass,

    #[error("struct has no members")]
    StructHasNoMembers,

    #[error("array has no element")]
    ArrayHasNoElement,

    #[error("array has no size")]
    ArrayHasNoSize,

    #[error("endpoint has an unexpected number of types")]
    UnexpectedNumberOfTypes,
}

impl TryFrom<&TypeDescription> for Type {
    type Error = TypeDescriptionError;

    fn try_from(
        TypeDescription {
            type_tag,
            members,
            element,
            size,
            class,
            ..
        }: &TypeDescription,
    ) -> Result<Self, Self::Error> {
        match type_tag {
            TypeTag::Void => Ok(Type::Primitive(Primitive::Void)),
            TypeTag::Bool => Ok(Type::Primitive(Primitive::Bool)),
            TypeTag::Int32 => Ok(Type::Primitive(Primitive::Int32)),
            TypeTag::Int64 => Ok(Type::Primitive(Primitive::Int64)),
            TypeTag::Float32 => Ok(Type::Primitive(Primitive::Float32)),
            TypeTag::Float64 => Ok(Type::Primitive(Primitive::Float64)),
            TypeTag::Object => {
                let class = class.clone().ok_or(Self::Error::StructHasNoClass)?;

                let mut object = Object::new(class);
                for (name, type_description) in
                    members.as_ref().ok_or(Self::Error::StructHasNoMembers)?
                {
                    object.add_field(name, Type::try_from(type_description)?);
                }
                Ok(object.into())
            }
            TypeTag::Array | TypeTag::Vector => {
                let element_ty: Type = element
                    .as_ref()
                    .ok_or(TypeDescriptionError::ArrayHasNoElement)?
                    .as_ref()
                    .try_into()?;
                let size = size.ok_or(TypeDescriptionError::ArrayHasNoSize)?;

                Ok(Array::new(element_ty, size).into())
            }
            TypeTag::String => Ok(Type::String),
        }
    }
}

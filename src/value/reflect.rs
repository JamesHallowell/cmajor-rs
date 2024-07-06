use {
    crate::value::{
        types::{Primitive, Type},
        Value,
    },
    serde::{de::Visitor, Deserialize, Deserializer},
    std::{any::TypeId, collections::VecDeque, fmt::Display},
};

pub(crate) trait Reflect: for<'de> Deserialize<'de> + 'static {
    fn reflect() -> Result<Option<Type>, Error>;
}

impl<T> Reflect for T
where
    T: for<'de> Deserialize<'de> + 'static,
{
    fn reflect() -> Result<Option<Type>, Error> {
        get_type::<T>()
    }
}

fn get_type<T>() -> Result<Option<Type>, Error>
where
    T: Reflect,
{
    if TypeId::of::<T>() == TypeId::of::<Value>() {
        return Ok(None);
    }

    let mut deserializer = TypeDeserializer {
        ty: Type::Primitive(Primitive::Void),
        fields: VecDeque::new(),
    };
    T::deserialize(&mut deserializer)?;
    Ok(Some(deserializer.ty))
}

struct TypeDeserializer {
    ty: Type,
    fields: VecDeque<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not supported")]
    NotSupported,

    #[error("unexpected field")]
    UnexpectedField,

    #[error("message: {0}")]
    Serde(String),
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Serde(msg.to_string())
    }
}

impl<'a, 'de> Deserializer<'de> for &'a mut TypeDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::NotSupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &mut self.ty {
            Type::Object(object) => {
                let field = self.fields.pop_front().ok_or(Error::UnexpectedField)?;
                object.add_field(field, Primitive::Bool);
            }
            _ => {
                self.ty = Type::Primitive(Primitive::Bool);
            }
        }

        visitor.visit_bool(bool::default())
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &mut self.ty {
            Type::Object(object) => {
                let field = self.fields.pop_front().ok_or(Error::UnexpectedField)?;
                object.add_field(field, Primitive::Int32);
            }
            _ => {
                self.ty = Type::Primitive(Primitive::Int32);
            }
        }

        visitor.visit_i32(i32::default())
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &mut self.ty {
            Type::Object(object) => {
                let field = self.fields.pop_front().ok_or(Error::UnexpectedField)?;
                object.add_field(field, Primitive::Int64);
            }
            _ => {
                self.ty = Type::Primitive(Primitive::Int64);
            }
        }

        visitor.visit_i64(i64::default())
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &mut self.ty {
            Type::Object(object) => {
                let field = self.fields.pop_front().ok_or(Error::UnexpectedField)?;
                object.add_field(field, Primitive::Float32);
            }
            _ => {
                self.ty = Type::Primitive(Primitive::Float32);
            }
        }

        visitor.visit_f32(f32::default())
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &mut self.ty {
            Type::Object(object) => {
                let field = self.fields.pop_front().ok_or(Error::UnexpectedField)?;
                object.add_field(field, Primitive::Float64);
            }
            _ => {
                self.ty = Type::Primitive(Primitive::Float64);
            }
        }

        visitor.visit_f64(f64::default())
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut s = TypeDeserializer {
            ty: Type::Object(Box::default()),
            fields: fields.iter().map(|s| s.to_string()).collect(),
        };
        let result = visitor.visit_seq(SequenceAccess { de: &mut s })?;

        match &mut self.ty {
            Type::Object(ref mut object) => {
                let field = self.fields.pop_front().ok_or(Error::UnexpectedField)?;
                object.add_field(field, s.ty);
            }
            _ => {
                self.ty = s.ty;
            }
        }

        Ok(result)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported)
    }
}

struct SequenceAccess<'a> {
    de: &'a mut TypeDeserializer,
}

impl<'a, 'de> serde::de::SeqAccess<'de> for SequenceAccess<'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de).map(Some)
    }
}

#[cfg(test)]
mod test {
    use {super::*, crate::value::Complex32, serde::Deserialize};

    #[test]
    fn primitive_types() {
        assert_eq!(
            get_type::<bool>().unwrap(),
            Some(Type::Primitive(Primitive::Bool))
        );
        assert_eq!(
            get_type::<i32>().unwrap(),
            Some(Type::Primitive(Primitive::Int32))
        );
        assert_eq!(
            get_type::<i64>().unwrap(),
            Some(Type::Primitive(Primitive::Int64))
        );
        assert_eq!(
            get_type::<f32>().unwrap(),
            Some(Type::Primitive(Primitive::Float32))
        );
        assert_eq!(
            get_type::<f64>().unwrap(),
            Some(Type::Primitive(Primitive::Float64))
        );
    }

    #[test]
    fn structs() {
        let ty = get_type::<Complex32>().unwrap().unwrap();
        let object = ty.as_object().unwrap();

        let fields = object.fields().collect::<Vec<_>>();
        assert_eq!(fields.len(), 2);

        assert_eq!(fields[0].name(), "real");
        assert_eq!(fields[0].ty(), &Type::Primitive(Primitive::Float32));
        assert_eq!(fields[0].offset(), 0);

        assert_eq!(fields[1].name(), "imag");
        assert_eq!(fields[1].ty(), &Type::Primitive(Primitive::Float32));
        assert_eq!(fields[1].offset(), 4);
    }

    #[test]
    fn nested_structs() {
        #[derive(Deserialize)]
        struct Outer {
            _inner: Inner,
        }

        #[derive(Deserialize)]
        struct Inner {
            _a: bool,
            _b: i32,
        }

        let ty = get_type::<Outer>().unwrap().unwrap();
        let object = ty.as_object().unwrap();

        let fields = object.fields().collect::<Vec<_>>();

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name(), "_inner");

        let inner = fields[0].ty().as_object().unwrap();
        let inner_fields = inner.fields().collect::<Vec<_>>();

        assert_eq!(inner_fields.len(), 2);
        assert_eq!(inner_fields[0].name(), "_a");
        assert_eq!(inner_fields[0].ty(), &Type::Primitive(Primitive::Bool));
        assert_eq!(inner_fields[1].name(), "_b");
        assert_eq!(inner_fields[1].ty(), &Type::Primitive(Primitive::Int32));
    }
}

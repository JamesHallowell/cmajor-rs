use {
    crate::value::types::{Array, Object, Type, TypeRef},
    bytes::Buf,
    smallvec::SmallVec,
};

#[derive(Debug, Clone)]
pub enum Value {
    Void,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Array(Box<ArrayValue>),
    Object(Box<ObjectValue>),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ValueRef<'a> {
    Void,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Array(ArrayValueRef<'a>),
    Object(ObjectValueRef<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayValue {
    ty: Array,
    data: SmallVec<[u8; 16]>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ArrayValueRef<'a> {
    ty: &'a Array,
    data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectValue {
    ty: Object,
    data: SmallVec<[u8; 16]>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ObjectValueRef<'a> {
    ty: &'a Object,
    data: &'a [u8],
}

impl Value {
    pub fn ty(&self) -> TypeRef<'_> {
        match self {
            Value::Void => TypeRef::Void,
            Value::Bool(_) => TypeRef::Bool,
            Value::Int32(_) => TypeRef::Int32,
            Value::Int64(_) => TypeRef::Int64,
            Value::Float32(_) => TypeRef::Float32,
            Value::Float64(_) => TypeRef::Float64,
            Value::Array(array) => TypeRef::Array(&array.ty),
            Value::Object(object) => TypeRef::Object(&object.ty),
        }
    }

    pub fn as_ref(&self) -> ValueRef<'_> {
        match self {
            Value::Void => ValueRef::Void,
            Value::Bool(value) => ValueRef::Bool(*value),
            Value::Int32(value) => ValueRef::Int32(*value),
            Value::Int64(value) => ValueRef::Int64(*value),
            Value::Float32(value) => ValueRef::Float32(*value),
            Value::Float64(value) => ValueRef::Float64(*value),
            Value::Array(ref array) => ValueRef::Array(array.as_ref().as_ref()),
            Value::Object(object) => ValueRef::Object(object.as_ref().as_ref()),
        }
    }

    pub fn with_bytes<R>(&self, callback: impl FnMut(&[u8]) -> R) -> R {
        self.as_ref().with_bytes(callback)
    }
}

impl<'a> ValueRef<'a> {
    pub fn new_from_slice<'b>(ty: TypeRef<'b>, mut data: &'b [u8]) -> ValueRef<'a>
    where
        'b: 'a,
    {
        match ty {
            TypeRef::Void => Self::Void,
            TypeRef::Bool => Self::Bool(data.get_u32_ne() != 0),
            TypeRef::Int32 => Self::Int32(data.get_i32_ne()),
            TypeRef::Int64 => Self::Int64(data.get_i64_ne()),
            TypeRef::Float32 => Self::Float32(data.get_f32_ne()),
            TypeRef::Float64 => Self::Float64(data.get_f64_ne()),
            TypeRef::Array(array) => Self::Array(ArrayValueRef::new_from_slice(array, data)),
            TypeRef::Object(object) => Self::Object(ObjectValueRef::new_from_slice(object, data)),
        }
    }

    pub fn ty(&self) -> TypeRef<'_> {
        match self {
            Self::Void => TypeRef::Void,
            Self::Bool(_) => TypeRef::Bool,
            Self::Int32(_) => TypeRef::Int32,
            Self::Int64(_) => TypeRef::Int64,
            Self::Float32(_) => TypeRef::Float32,
            Self::Float64(_) => TypeRef::Float64,
            Self::Array(array) => TypeRef::Array(array.ty),
            Self::Object(object) => TypeRef::Object(object.ty),
        }
    }

    pub fn to_owned(&self) -> Value {
        match *self {
            Self::Void => Value::from(()),
            Self::Bool(value) => Value::from(value),
            Self::Int32(value) => Value::from(value),
            Self::Int64(value) => Value::from(value),
            Self::Float32(value) => Value::from(value),
            Self::Float64(value) => Value::from(value),
            Self::Array(array) => Value::from(array.to_owned()),
            Self::Object(object) => Value::from(object.to_owned()),
        }
    }

    pub fn with_bytes<R>(&self, mut callback: impl FnMut(&[u8]) -> R) -> R {
        match *self {
            Self::Void => callback(&[]),
            Self::Bool(value) => callback((value as u32).to_ne_bytes().as_slice()),
            Self::Int32(value) => callback(value.to_ne_bytes().as_slice()),
            Self::Int64(value) => callback(value.to_ne_bytes().as_slice()),
            Self::Float32(value) => callback(value.to_ne_bytes().as_slice()),
            Self::Float64(value) => callback(value.to_ne_bytes().as_slice()),
            Self::Array(array) => callback(array.data),
            Self::Object(object) => callback(object.data),
        }
    }
}

impl ArrayValue {
    pub fn as_ref(&self) -> ArrayValueRef<'_> {
        ArrayValueRef {
            ty: &self.ty,
            data: &self.data,
        }
    }
}

impl<'a> ArrayValueRef<'a> {
    pub(crate) fn new_from_slice<'b>(ty: &'b Array, data: &'b [u8]) -> ArrayValueRef<'a>
    where
        'b: 'a,
    {
        Self {
            ty,
            data: &data[..ty.size()],
        }
    }

    pub fn get(&'a self, index: usize) -> Option<ValueRef<'a>> {
        if index >= self.len() {
            return None;
        }

        let ty = self.elem_ty();
        let offset = ty.size() * index;
        let data = &self.data[offset..offset + ty.size()];

        Some(ValueRef::new_from_slice(ty.as_ref(), data))
    }

    pub fn elem_ty(&self) -> &Type {
        self.ty.elem_ty()
    }

    pub fn len(&self) -> usize {
        self.ty.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_owned(&self) -> ArrayValue {
        ArrayValue {
            ty: self.ty.clone(),
            data: SmallVec::from_slice(self.data),
        }
    }
}

impl ObjectValue {
    pub fn as_ref(&self) -> ObjectValueRef<'_> {
        ObjectValueRef {
            ty: &self.ty,
            data: &self.data,
        }
    }
}

impl<'a> ObjectValueRef<'a> {
    pub(crate) fn new_from_slice<'b>(ty: &'b Object, data: &'b [u8]) -> ObjectValueRef<'a>
    where
        'b: 'a,
    {
        Self {
            ty,
            data: &data[..ty.size()],
        }
    }

    pub fn field(&self, name: impl AsRef<str>) -> Option<ValueRef<'_>> {
        let mut offset = 0;
        self.ty
            .fields()
            .find_map(|field| {
                (field.name() == name.as_ref())
                    .then_some((field, offset))
                    .or_else(|| {
                        offset += field.ty().size();
                        None
                    })
            })
            .map(|(field, offset)| {
                ValueRef::new_from_slice(field.ty().as_ref(), &self.data[offset..])
            })
    }

    pub fn to_owned(&self) -> ObjectValue {
        ObjectValue {
            ty: self.ty.clone(),
            data: SmallVec::from_slice(self.data),
        }
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Self::Void
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Int32(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int64(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float32(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float64(value)
    }
}

impl From<ArrayValue> for Value {
    fn from(array: ArrayValue) -> Self {
        Self::Array(Box::new(array))
    }
}

impl From<ObjectValue> for Value {
    fn from(object: ObjectValue) -> Self {
        Self::Object(Box::new(object))
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Complex32 {
    pub imag: f32,
    pub real: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Complex64 {
    pub imag: f64,
    pub real: f64,
}

impl From<Complex32> for Value {
    fn from(value: Complex32) -> Self {
        let object = Object::new()
            .with_field("imag", Type::Float32)
            .with_field("real", Type::Float32);

        let mut data = SmallVec::new();
        data.extend_from_slice(&value.imag.to_ne_bytes());
        data.extend_from_slice(&value.real.to_ne_bytes());

        ObjectValue { ty: object, data }.into()
    }
}

impl TryFrom<ValueRef<'_>> for Complex32 {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Object(object) => match (object.field("imag"), object.field("real")) {
                (Some(ValueRef::Float32(imag)), Some(ValueRef::Float32(real))) => {
                    Ok(Self { imag, real })
                }
                _ => Err(()),
            },
            _ => Err(()),
        }
    }
}

impl From<Complex64> for Value {
    fn from(value: Complex64) -> Self {
        let object = Object::new()
            .with_field("imag", Type::Float64)
            .with_field("real", Type::Float64);

        let mut data = SmallVec::new();
        data.extend_from_slice(&value.imag.to_ne_bytes());
        data.extend_from_slice(&value.real.to_ne_bytes());

        ObjectValue { ty: object, data }.into()
    }
}

impl TryFrom<ValueRef<'_>> for Complex64 {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Object(object) => match (object.field("imag"), object.field("real")) {
                (Some(ValueRef::Float64(imag)), Some(ValueRef::Float64(real))) => {
                    Ok(Self { imag, real })
                }
                _ => Err(()),
            },
            _ => Err(()),
        }
    }
}

impl<T, const N: usize> From<[T; N]> for Value
where
    T: Into<Value> + Default,
{
    fn from(value: [T; N]) -> Self {
        let v = T::default().into();
        let ty = v.ty();

        let array = Array::new(ty.to_owned(), N);
        let mut data = SmallVec::new();
        for value in value {
            let value: Value = value.into();
            value.with_bytes(|bytes| {
                data.extend_from_slice(bytes);
            });
        }
        ArrayValue { ty: array, data }.into()
    }
}

impl<'a> From<&'a Value> for ValueRef<'a> {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Void => Self::Void,
            Value::Bool(value) => Self::Bool(*value),
            Value::Int32(value) => Self::Int32(*value),
            Value::Int64(value) => Self::Int64(*value),
            Value::Float32(value) => Self::Float32(*value),
            Value::Float64(value) => Self::Float64(*value),
            Value::Array(array) => Self::Array(array.as_ref().as_ref()),
            Value::Object(object) => Self::Object(object.as_ref().as_ref()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bool_as_value() {
        let value: Value = true.into();
        assert!(matches!(value.as_ref(), ValueRef::Bool(true)));
    }

    #[test]
    fn int32_as_value() {
        let value: Value = 5_i32.into();
        assert!(matches!(value.as_ref(), ValueRef::Int32(5)));
    }

    #[test]
    fn int64_as_value() {
        let value: Value = 5_i64.into();
        assert!(matches!(value.as_ref(), ValueRef::Int64(5)));
    }

    #[test]
    fn float32_as_value() {
        let value: Value = 5.0_f32.into();
        assert!(matches!(value.as_ref(), ValueRef::Float32(value) if value == 5.0_f32));
    }

    #[test]
    fn float64_as_value() {
        let value: Value = 5.0_f64.into();
        assert!(matches!(value.as_ref(), ValueRef::Float64(value) if value == 5.0_f64));
    }

    #[test]
    fn array_as_value() {
        let array: Type = Array::new(Type::Int32, 3).into();
        assert_eq!(array.size(), 12);

        let values = [5, 6, 7];

        let value: Value = values.into();

        let array_view = match value.as_ref() {
            ValueRef::Array(array_view) => array_view,
            _ => panic!("Expected array"),
        };

        assert_eq!(array_view.len(), 3);
        assert!(!array_view.is_empty());

        assert_eq!(array_view.get(0), Some(ValueRef::Int32(5)));
        assert_eq!(array_view.get(1), Some(ValueRef::Int32(6)));
        assert_eq!(array_view.get(2), Some(ValueRef::Int32(7)));
    }

    #[test]
    fn multi_dimensional_array_as_value() {
        let array: Type = Array::new(Array::new(Type::Int32, 3), 2).into();
        assert_eq!(array.size(), 24);

        let multi_dimensional_array = [[5, 6, 7], [8, 9, 10]];
        let value: Value = multi_dimensional_array.into();

        let outer = match value.as_ref() {
            ValueRef::Array(array_view) => array_view,
            _ => panic!("Expected array"),
        };
        assert_eq!(outer.len(), 2);

        let inner = match outer.get(0) {
            Some(ValueRef::Array(inner)) => inner,
            _ => panic!("Expected array"),
        };
        assert_eq!(inner.get(0), Some(ValueRef::Int32(5)));
        assert_eq!(inner.get(1), Some(ValueRef::Int32(6)));
        assert_eq!(inner.get(2), Some(ValueRef::Int32(7)));
        assert_eq!(inner.get(3), None);

        let inner = match outer.get(1) {
            Some(ValueRef::Array(inner)) => inner,
            _ => panic!("Expected array"),
        };

        assert_eq!(inner.get(0), Some(ValueRef::Int32(8)));
        assert_eq!(inner.get(1), Some(ValueRef::Int32(9)));
        assert_eq!(inner.get(2), Some(ValueRef::Int32(10)));
        assert_eq!(inner.get(3), None);

        assert_eq!(outer.get(3), None);
    }

    #[test]
    fn object_as_value() {
        let ty = Object::new()
            .with_field("a", Type::Int32)
            .with_field("b", Type::Int64)
            .with_field("c", Object::new().with_field("d", Type::Bool));

        let mut data = Vec::new();
        data.extend_from_slice(&5_i32.to_ne_bytes());
        data.extend_from_slice(&53_i64.to_ne_bytes());
        data.extend_from_slice(&1_i32.to_ne_bytes());

        let object = ObjectValueRef::new_from_slice(&ty, &data);

        assert_eq!(object.field("a"), Some(ValueRef::Int32(5)));
        assert_eq!(object.field("b"), Some(ValueRef::Int64(53)));

        let inner = match object.field("c") {
            Some(ValueRef::Object(inner)) => inner,
            _ => panic!("Expected object"),
        };

        assert_eq!(inner.field("d"), Some(ValueRef::Bool(true)));
    }

    #[test]
    fn value_is_16_bytes() {
        assert_eq!(std::mem::size_of::<Value>(), 16);
    }
}

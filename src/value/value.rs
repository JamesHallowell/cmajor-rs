use {
    crate::value::types::{Array, Object, Type},
    bytes::Buf,
    smallvec::SmallVec,
};

#[derive(Debug, Clone)]
pub struct Value {
    ty: Type,
    data: Data,
}

type Data = SmallVec<[u8; 8]>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ValueRef<'a> {
    Void,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Array(ArrayRef<'a>),
    Object(ObjectRef<'a>),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ArrayRef<'a> {
    ty: &'a Array,
    data: &'a [u8],
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ObjectRef<'a> {
    ty: &'a Object,
    data: &'a [u8],
}

impl Value {
    fn new(ty: impl Into<Type>, data: Data) -> Self {
        Value {
            ty: ty.into(),
            data,
        }
    }

    pub fn get(&self) -> ValueRef<'_> {
        ValueRef::new(&self.ty, &self.data)
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl<'a> ValueRef<'a> {
    pub fn new<'b>(ty: &'b Type, data: &'b [u8]) -> ValueRef<'a>
    where
        'b: 'a,
    {
        let mut data = data;
        match ty {
            Type::Void => Self::Void,
            Type::Bool => Self::Bool(data.get_u32_ne() != 0),
            Type::Int32 => Self::Int32(data.get_i32_ne()),
            Type::Int64 => Self::Int64(data.get_i64_ne()),
            Type::Float32 => Self::Float32(data.get_f32_ne()),
            Type::Float64 => Self::Float64(data.get_f64_ne()),
            Type::Array(ref array) => Self::Array(ArrayRef {
                ty: array.as_ref(),
                data,
            }),
            Type::Object(ref object) => Self::Object(ObjectRef {
                ty: object.as_ref(),
                data,
            }),
        }
    }

    pub fn object(&'a self) -> Option<ObjectRef<'a>> {
        match *self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn array(&'a self) -> Option<ArrayRef<'a>> {
        match *self {
            Self::Array(array) => Some(array),
            _ => None,
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
            Self::Array(array) => Value::from(array),
            Self::Object(object) => Value::from(object),
        }
    }
}
impl<'a> ArrayRef<'a> {
    pub fn get(&'a self, index: usize) -> Option<ValueRef<'a>> {
        if index >= self.len() {
            return None;
        }

        let ty = self.elem_ty();
        let offset = ty.size() * index;
        let data = &self.data[offset..offset + ty.size()];

        Some(ValueRef::new(ty, data))
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
}

impl ObjectRef<'_> {
    pub fn field(&self, name: impl AsRef<str>) -> Option<ValueRef<'_>> {
        let name = name.as_ref();
        let mut offset = 0;
        for field in self.ty.fields() {
            if field.name() == name {
                return Some(ValueRef::new(field.ty(), &self.data[offset..]));
            }
            offset += field.ty().size();
        }
        None
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value {
            ty: Type::Void,
            data: Data::new(),
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        let value: u32 = if value { 1 } else { 0 };
        Value {
            ty: Type::Bool,
            data: Data::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value {
            ty: Type::Int32,
            data: Data::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value {
            ty: Type::Int64,
            data: Data::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value {
            ty: Type::Float32,
            data: Data::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value {
            ty: Type::Float64,
            data: Data::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<ArrayRef<'_>> for Value {
    fn from(array: ArrayRef<'_>) -> Self {
        Value::new(array.ty.clone(), Data::from_slice(array.data))
    }
}

impl From<ObjectRef<'_>> for Value {
    fn from(object: ObjectRef<'_>) -> Self {
        Value::new(object.ty.clone(), Data::from_slice(object.data))
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

        let mut data = Data::new();
        data.extend_from_slice(&value.imag.to_ne_bytes());
        data.extend_from_slice(&value.real.to_ne_bytes());

        Self::new(object, data)
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
                _ => return Err(()),
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

        let mut data = Data::new();
        data.extend_from_slice(&value.imag.to_ne_bytes());
        data.extend_from_slice(&value.real.to_ne_bytes());

        Self::new(object, data)
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
                _ => return Err(()),
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
        let ty = T::default().into().ty().clone();

        let array = Array::new(ty, N);
        let mut data = Data::new();
        for value in value {
            let value: Value = value.into();
            data.extend_from_slice(value.data());
        }
        Self::new(array, data)
    }
}

impl<'a> From<&'a Value> for ValueRef<'a> {
    fn from(value: &'a Value) -> Self {
        Self::new(&value.ty, &value.data)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bool_as_value() {
        let value: Value = true.into();
        assert!(matches!(value.get(), ValueRef::Bool(true)));
    }

    #[test]
    fn int32_as_value() {
        let value: Value = 5_i32.into();
        assert!(matches!(value.get(), ValueRef::Int32(5)));
    }

    #[test]
    fn int64_as_value() {
        let value: Value = 5_i64.into();
        assert!(matches!(value.get(), ValueRef::Int64(5)));
    }

    #[test]
    fn float32_as_value() {
        let value: Value = 5.0_f32.into();
        assert!(matches!(value.get(), ValueRef::Float32(value) if value == 5.0_f32));
    }

    #[test]
    fn float64_as_value() {
        let value: Value = 5.0_f64.into();
        assert!(matches!(value.get(), ValueRef::Float64(value) if value == 5.0_f64));
    }

    #[test]
    fn array_as_value() {
        let array: Type = Array::new(Type::Int32, 3).into();
        assert_eq!(array.size(), 12);

        let values = [5, 6, 7];

        let value: Value = values.into();

        let array_view = match value.get() {
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

        let outer = match value.get() {
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

        let mut inner = match outer.get(1) {
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
        let object: Type = Object::new()
            .with_field("a", Type::Int32)
            .with_field("b", Type::Int64)
            .with_field("c", Object::new().with_field("d", Type::Bool))
            .into();

        let mut data = Data::new();
        data.extend_from_slice(&5_i32.to_ne_bytes());
        data.extend_from_slice(&53_i64.to_ne_bytes());
        data.extend_from_slice(&1_i32.to_ne_bytes());

        let value = Value::new(object, data);

        let object_view = match value.get() {
            ValueRef::Object(object_view) => object_view,
            _ => panic!("Expected object"),
        };

        assert!(matches!(object_view.field("a"), Some(ValueRef::Int32(5))));
        assert!(matches!(object_view.field("b"), Some(ValueRef::Int64(53))));

        let inner = match object_view.field("c") {
            Some(ValueRef::Object(inner)) => inner,
            _ => panic!("Expected object"),
        };

        assert!(matches!(inner.field("d"), Some(ValueRef::Bool(true))));
    }
}

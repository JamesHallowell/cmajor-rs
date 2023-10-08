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

#[derive(Debug, Copy, Clone)]
pub struct ValueRef<'a> {
    ty: &'a Type,
    data: &'a [u8],
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ValueView<'a> {
    Void,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Array(ArrayView<'a>),
    Object(ObjectView<'a>),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ArrayView<'a> {
    array: &'a Array,
    data: &'a [u8],
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ObjectView<'a> {
    object: &'a Object,
    data: &'a [u8],
}

impl Value {
    fn new(ty: impl Into<Type>, data: Data) -> Self {
        Value {
            ty: ty.into(),
            data,
        }
    }

    pub fn get(&self) -> ValueView<'_> {
        ValueView::new(&self.ty, &self.data)
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl<'a> ValueRef<'a> {
    pub fn from_bytes<'b>(ty: &'b Type, data: &'b [u8]) -> ValueRef<'a>
    where
        'b: 'a,
    {
        Self {
            ty,
            data: &data[..ty.size()],
        }
    }

    pub fn get(&self) -> ValueView<'_> {
        ValueView::new(self.ty, self.data)
    }

    pub fn ty(&self) -> &Type {
        self.ty
    }

    pub fn data(&self) -> &[u8] {
        self.data
    }

    pub fn object(&'a self) -> Option<ObjectView<'a>> {
        match self.get() {
            ValueView::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn array(&'a self) -> Option<ArrayView<'a>> {
        match self.get() {
            ValueView::Array(array) => Some(array),
            _ => None,
        }
    }

    pub fn to_owned(&self) -> Value {
        Value {
            ty: self.ty.clone(),
            data: self.data.to_vec().into(),
        }
    }
}

impl<'a> ValueView<'a> {
    fn new<'b>(value_type: &'b Type, data: &'b [u8]) -> Self
    where
        'b: 'a,
    {
        let mut data = data;
        match value_type {
            Type::Void => ValueView::Void,
            Type::Bool => ValueView::Bool(data.get_u32_ne() != 0),
            Type::Int32 => ValueView::Int32(data.get_i32_ne()),
            Type::Int64 => ValueView::Int64(data.get_i64_ne()),
            Type::Float32 => ValueView::Float32(data.get_f32_ne()),
            Type::Float64 => ValueView::Float64(data.get_f64_ne()),
            Type::Array(ref array) => ValueView::Array(ArrayView { array, data }),
            Type::Object(ref object) => ValueView::Object(ObjectView { object, data }),
        }
    }
}

impl<'a> ArrayView<'a> {
    pub fn get(&self, index: usize) -> Option<ValueView<'a>> {
        if index >= self.array.len() {
            return None;
        }

        let offset = self.array.elem_ty().size() * index;
        let data = &self.data[offset..offset + self.array.elem_ty().size()];

        Some(ValueView::new(self.array.elem_ty(), data))
    }

    pub fn len(&self) -> usize {
        self.array.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct ArrayViewIterator<'a> {
    array: ArrayView<'a>,
    index: usize,
}

impl<'a> Iterator for ArrayViewIterator<'a> {
    type Item = ValueView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.array.get(self.index);
        self.index += 1;
        value
    }
}

impl<'a> IntoIterator for ArrayView<'a> {
    type Item = ValueView<'a>;
    type IntoIter = ArrayViewIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ArrayViewIterator {
            array: self,
            index: 0,
        }
    }
}

impl ObjectView<'_> {
    pub fn field(&self, name: impl AsRef<str>) -> Option<ValueView<'_>> {
        let name = name.as_ref();
        let mut offset = 0;
        for field in self.object.fields() {
            if field.name() == name {
                return Some(ValueView::new(field.ty(), &self.data[offset..]));
            }
            offset += field.ty().size();
        }
        None
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
        match value.get() {
            ValueView::Object(object) => match (object.field("imag"), object.field("real")) {
                (Some(ValueView::Float32(imag)), Some(ValueView::Float32(real))) => {
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
        match value.get() {
            ValueView::Object(object) => match (object.field("imag"), object.field("real")) {
                (Some(ValueView::Float64(imag)), Some(ValueView::Float64(real))) => {
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
        ValueRef {
            ty: &value.ty,
            data: &value.data,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bool_as_value() {
        let value: Value = true.into();
        assert!(matches!(value.get(), ValueView::Bool(true)));
    }

    #[test]
    fn int32_as_value() {
        let value: Value = 5_i32.into();
        assert!(matches!(value.get(), ValueView::Int32(5)));
    }

    #[test]
    fn int64_as_value() {
        let value: Value = 5_i64.into();
        assert!(matches!(value.get(), ValueView::Int64(5)));
    }

    #[test]
    fn float32_as_value() {
        let value: Value = 5.0_f32.into();
        assert!(matches!(value.get(), ValueView::Float32(value) if value == 5.0_f32));
    }

    #[test]
    fn float64_as_value() {
        let value: Value = 5.0_f64.into();
        assert!(matches!(value.get(), ValueView::Float64(value) if value == 5.0_f64));
    }

    #[test]
    fn array_as_value() {
        let array: Type = Array::new(Type::Int32, 3).into();
        assert_eq!(array.size(), 12);

        let values = [5, 6, 7];

        let value: Value = values.into();

        let array_view = match value.get() {
            ValueView::Array(array_view) => array_view,
            _ => panic!("Expected array"),
        };

        assert_eq!(array_view.len(), 3);
        assert!(!array_view.is_empty());

        let mut iter = array_view.into_iter();
        assert!(matches!(iter.next(), Some(ValueView::Int32(5))));
        assert!(matches!(iter.next(), Some(ValueView::Int32(6))));
        assert!(matches!(iter.next(), Some(ValueView::Int32(7))));
    }

    #[test]
    fn multi_dimensional_array_as_value() {
        let array: Type = Array::new(Array::new(Type::Int32, 3), 2).into();
        assert_eq!(array.size(), 24);

        let multi_dimensional_array = [[5, 6, 7], [8, 9, 10]];
        let value: Value = multi_dimensional_array.into();

        let array_view = match value.get() {
            ValueView::Array(array_view) => array_view,
            _ => panic!("Expected array"),
        };
        assert_eq!(array_view.len(), 2);

        let mut outer = array_view.into_iter();

        let mut inner = match outer.next() {
            Some(ValueView::Array(inner)) => inner.into_iter(),
            _ => panic!("Expected array"),
        };
        assert!(matches!(inner.next(), Some(ValueView::Int32(5))));
        assert!(matches!(inner.next(), Some(ValueView::Int32(6))));
        assert!(matches!(inner.next(), Some(ValueView::Int32(7))));
        assert!(matches!(inner.next(), None));

        let mut inner = match outer.next() {
            Some(ValueView::Array(inner)) => inner.into_iter(),
            _ => panic!("Expected array"),
        };

        assert!(matches!(inner.next(), Some(ValueView::Int32(8))));
        assert!(matches!(inner.next(), Some(ValueView::Int32(9))));
        assert!(matches!(inner.next(), Some(ValueView::Int32(10))));
        assert!(matches!(inner.next(), None));

        assert!(matches!(outer.next(), None));
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
            ValueView::Object(object_view) => object_view,
            _ => panic!("Expected object"),
        };

        assert!(matches!(object_view.field("a"), Some(ValueView::Int32(5))));
        assert!(matches!(object_view.field("b"), Some(ValueView::Int64(53))));

        let inner = match object_view.field("c") {
            Some(ValueView::Object(inner)) => inner,
            _ => panic!("Expected object"),
        };

        assert!(matches!(inner.field("d"), Some(ValueView::Bool(true))));
    }
}

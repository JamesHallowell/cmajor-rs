use {bytes::Buf, smallvec::SmallVec};

mod types;
pub use types::{Array, IsType, Object, Type};

pub struct Value {
    ty: Type,
    data: SmallVec<[u8; 8]>,
}

#[derive(Copy, Clone)]
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
    pub fn new(ty: impl Into<Type>, data: &[u8]) -> Self {
        Value {
            ty: ty.into(),
            data: SmallVec::from_slice(data),
        }
    }

    pub fn get(&self) -> ValueView<'_> {
        ValueView::new(&self.ty, &self.data)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }
}

impl<'a> ValueRef<'a> {
    pub fn new<'b>(ty: &'b Type, data: &'b [u8]) -> ValueRef<'a>
    where
        'b: 'a,
    {
        Self { ty, data }
    }

    pub fn get(&self) -> ValueView<'_> {
        ValueView::new(self.ty, self.data)
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

        let offset = self.array.ty().size() * index;
        let data = &self.data[offset..offset + self.array.ty().size()];

        Some(ValueView::new(self.array.ty(), data))
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
            data: SmallVec::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value {
            ty: Type::Int32,
            data: SmallVec::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value {
            ty: Type::Int64,
            data: SmallVec::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value {
            ty: Type::Float32,
            data: SmallVec::from_slice(&value.to_ne_bytes()),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value {
            ty: Type::Float64,
            data: SmallVec::from_slice(&value.to_ne_bytes()),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Complex32 {
    pub imag: f32,
    pub real: f32,
}

impl From<Complex32> for Value {
    fn from(value: Complex32) -> Self {
        let object = Object::new()
            .with_field("imag", Type::Float32)
            .with_field("real", Type::Float32);

        Self::new(
            object,
            &[value.imag.to_ne_bytes(), value.real.to_ne_bytes()].concat(),
        )
    }
}

impl<'a> From<&'a Value> for ValueRef<'a> {
    fn from(value: &'a Value) -> Self {
        ValueRef::new(&value.ty, &value.data)
    }
}

#[cfg(test)]
mod test {
    use {super::*, bytes::BufMut};

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

        let mut data = vec![0; array.size()];
        let mut buf = data.as_mut_slice();
        buf.put_i32_ne(5);
        buf.put_i32_ne(6);
        buf.put_i32_ne(7);

        let value = Value::new(array, &data);

        let array_view = match value.get() {
            ValueView::Array(array_view) => array_view,
            _ => panic!("Expected array"),
        };

        let mut iter = array_view.into_iter();
        assert!(matches!(iter.next(), Some(ValueView::Int32(5))));
        assert!(matches!(iter.next(), Some(ValueView::Int32(6))));
        assert!(matches!(iter.next(), Some(ValueView::Int32(7))));
    }

    #[test]
    fn multi_dimensional_array_as_value() {
        let array: Type = Array::new(Array::new(Type::Int32, 3), 2).into();
        assert_eq!(array.size(), 24);

        let mut data = vec![0; array.size()];
        let mut buf = data.as_mut_slice();
        buf.put_i32_ne(5);
        buf.put_i32_ne(6);
        buf.put_i32_ne(7);
        buf.put_i32_ne(8);
        buf.put_i32_ne(9);
        buf.put_i32_ne(10);

        let value = Value::new(array, &data);

        let mut outer = match value.get() {
            ValueView::Array(array_view) => array_view.into_iter(),
            _ => panic!("Expected array"),
        };

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

        let mut data = vec![0; object.size()];
        let mut buf = data.as_mut_slice();
        buf.put_i32_ne(5);
        buf.put_i64_ne(53);
        buf.put_u32_ne(1);

        let value = Value::new(object, &data);

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

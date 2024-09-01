use {
    crate::value::types::{Array, IsFloatingPoint, Object, Primitive, Type, TypeRef},
    bytes::{Buf, BufMut},
    serde::{Deserialize, Serialize},
    smallvec::SmallVec,
};

/// A Cmajor value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// A void value.
    Void,

    /// A boolean value.
    Bool(bool),

    /// A 32-bit signed integer value.
    Int32(i32),

    /// A 64-bit signed integer value.
    Int64(i64),

    /// A 32-bit floating-point value.
    Float32(f32),

    /// A 64-bit floating-point value.
    Float64(f64),

    /// An array value.
    Array(Box<ArrayValue>),

    /// An object value.
    Object(Box<ObjectValue>),
}

/// A reference to a [`Value`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ValueRef<'a> {
    /// A void value.
    Void,

    /// A boolean value.
    Bool(bool),

    /// A 32-bit signed integer value.
    Int32(i32),

    /// A 64-bit signed integer value.
    Int64(i64),

    /// A 32-bit floating-point value.
    Float32(f32),

    /// A 64-bit floating-point value.
    Float64(f64),

    /// An array value.
    Array(ArrayValueRef<'a>),

    /// An object value.
    Object(ObjectValueRef<'a>),
}

/// An array value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayValue {
    ty: Array,
    data: SmallVec<[u8; 16]>,
}

/// A reference to an [`ArrayValue`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ArrayValueRef<'a> {
    ty: &'a Array,
    data: &'a [u8],
}

/// An object value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectValue {
    ty: Object,
    data: SmallVec<[u8; 16]>,
}

/// A reference to an [`ObjectValue`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ObjectValueRef<'a> {
    ty: &'a Object,
    data: &'a [u8],
}

impl Value {
    /// Get the type of the value.
    pub fn ty(&self) -> TypeRef<'_> {
        match self {
            Self::Void => TypeRef::Primitive(Primitive::Void),
            Self::Bool(_) => TypeRef::Primitive(Primitive::Bool),
            Self::Int32(_) => TypeRef::Primitive(Primitive::Int32),
            Self::Int64(_) => TypeRef::Primitive(Primitive::Int64),
            Self::Float32(_) => TypeRef::Primitive(Primitive::Float32),
            Self::Float64(_) => TypeRef::Primitive(Primitive::Float64),
            Self::Array(array) => TypeRef::Array(&array.ty),
            Self::Object(object) => TypeRef::Object(&object.ty),
        }
    }

    /// Get a reference to the value.
    pub fn as_ref(&self) -> ValueRef<'_> {
        match self {
            Self::Void => ValueRef::Void,
            Self::Bool(value) => ValueRef::Bool(*value),
            Self::Int32(value) => ValueRef::Int32(*value),
            Self::Int64(value) => ValueRef::Int64(*value),
            Self::Float32(value) => ValueRef::Float32(*value),
            Self::Float64(value) => ValueRef::Float64(*value),
            Self::Array(ref array) => ValueRef::Array(array.as_ref().as_ref()),
            Self::Object(object) => ValueRef::Object(object.as_ref().as_ref()),
        }
    }

    pub(crate) fn with_bytes<R>(&self, callback: impl FnMut(&[u8]) -> R) -> R {
        self.as_ref().with_bytes(callback)
    }

    pub(crate) fn serialise_as_choc_value(&self) -> Vec<u8> {
        let mut serialised = Vec::new();
        serialised.put_slice(self.ty().serialise_as_choc_type().as_slice());
        self.with_bytes(|bytes| {
            serialised.put_slice(bytes);
        });
        serialised
    }
}

impl<'a> ValueRef<'a> {
    pub(crate) fn new_from_slice<'b>(ty: TypeRef<'b>, mut data: &'b [u8]) -> ValueRef<'a>
    where
        'b: 'a,
    {
        match ty {
            TypeRef::Primitive(Primitive::Void) => Self::Void,
            TypeRef::Primitive(Primitive::Bool) => Self::Bool(data.get_u32_ne() != 0),
            TypeRef::Primitive(Primitive::Int32) => Self::Int32(data.get_i32_ne()),
            TypeRef::Primitive(Primitive::Int64) => Self::Int64(data.get_i64_ne()),
            TypeRef::Primitive(Primitive::Float32) => Self::Float32(data.get_f32_ne()),
            TypeRef::Primitive(Primitive::Float64) => Self::Float64(data.get_f64_ne()),
            TypeRef::Array(array) => Self::Array(ArrayValueRef::new_from_slice(array, data)),
            TypeRef::Object(object) => Self::Object(ObjectValueRef::new_from_slice(object, data)),
        }
    }

    /// If the value is an array, get a reference to it. Otherwise returns `None`.
    pub fn as_array(&self) -> Option<ArrayValueRef<'_>> {
        match self {
            Self::Array(array) => Some(*array),
            _ => None,
        }
    }

    /// If the value is an object, get a reference to it. Otherwise, returns `None`.
    pub fn as_object(&self) -> Option<ObjectValueRef<'_>> {
        match self {
            Self::Object(object) => Some(*object),
            _ => None,
        }
    }

    /// Get the type of the value.
    pub fn ty(&self) -> TypeRef<'_> {
        match self {
            Self::Void => TypeRef::Primitive(Primitive::Void),
            Self::Bool(_) => TypeRef::Primitive(Primitive::Bool),
            Self::Int32(_) => TypeRef::Primitive(Primitive::Int32),
            Self::Int64(_) => TypeRef::Primitive(Primitive::Int64),
            Self::Float32(_) => TypeRef::Primitive(Primitive::Float32),
            Self::Float64(_) => TypeRef::Primitive(Primitive::Float64),
            Self::Array(array) => TypeRef::Array(array.ty),
            Self::Object(object) => TypeRef::Object(object.ty),
        }
    }

    /// Clone the value into an owned [`Value`].
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

    pub(crate) fn with_bytes<R>(&self, mut callback: impl FnMut(&[u8]) -> R) -> R {
        match *self {
            Self::Void => callback(&[]),
            Self::Bool(value) => callback(u32::from(value).to_ne_bytes().as_slice()),
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
    /// Get a reference to the array.
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

    /// Get the value at the given index. Returns `None` if the index is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// # use cmajor::value::{ArrayValue, ValueRef};
    /// let array: ArrayValue = [1, 2, 3].into();
    /// let array_ref = array.as_ref();
    ///
    /// assert_eq!(array_ref.get(0), Some(ValueRef::Int32(1)));
    /// ```
    pub fn get(&'a self, index: usize) -> Option<ValueRef<'a>> {
        if index >= self.len() {
            return None;
        }

        let ty = self.elem_ty();
        let offset = ty.size() * index;
        let data = &self.data[offset..offset + ty.size()];

        Some(ValueRef::new_from_slice(ty.as_ref(), data))
    }

    /// Returns an iterator over the array's elements.
    ///
    /// # Example
    ///
    /// ```
    /// # use cmajor::value::{ArrayValue, ValueRef};
    /// let array: ArrayValue = [1, 2, 3].into();
    /// let array_ref = array.as_ref();
    ///
    /// let mut iter = array_ref.elems();
    /// assert_eq!(iter.next(), Some(ValueRef::Int32(1)));
    /// assert_eq!(iter.next(), Some(ValueRef::Int32(2)));
    /// assert_eq!(iter.next(), Some(ValueRef::Int32(3)));
    /// assert_eq!(iter.next(), None);
    pub fn elems(&self) -> impl Iterator<Item = ValueRef<'_>> + '_ {
        (0..self.len()).filter_map(move |index| self.get(index))
    }

    /// Get the type of the array's elements.
    ///
    /// # Example
    ///
    /// ```
    /// # use cmajor::value::{ArrayValue, types::{Type, Primitive}};
    /// let array: ArrayValue = [1, 2, 3].into();
    /// let array_ref = array.as_ref();
    ///
    /// assert_eq!(array_ref.elem_ty(), &Type::Primitive(Primitive::Int32));
    pub fn elem_ty(&self) -> &Type {
        self.ty.elem_ty()
    }

    /// The number of elements in the array.
    ///
    /// # Example
    ///
    /// ```
    /// # use cmajor::value::{ArrayValue, ValueRef};
    /// let array: ArrayValue = [1, 2, 3].into();
    /// let array_ref = array.as_ref();
    ///
    /// assert_eq!(array_ref.len(), 3);
    pub fn len(&self) -> usize {
        self.ty.len()
    }

    /// Whether the array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clone into an owned [`ArrayValue`].
    pub fn to_owned(&self) -> ArrayValue {
        ArrayValue {
            ty: self.ty.clone(),
            data: SmallVec::from_slice(self.data),
        }
    }
}

impl ObjectValue {
    /// Get a reference to the object.
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

    /// Get the value of the given field. Returns `None` if the field does not exist.
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

    /// Returns an iterator over the object's fields.
    pub fn fields(&self) -> impl Iterator<Item = (&str, ValueRef<'_>)> + '_ {
        self.ty
            .fields()
            .filter_map(|field| self.field(field.name()).map(|value| (field.name(), value)))
    }

    /// Clone into an owned [`ObjectValue`].
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

impl From<()> for ValueRef<'_> {
    fn from(_: ()) -> Self {
        Self::Void
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<bool> for ValueRef<'_> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Int32(value)
    }
}

impl From<i32> for ValueRef<'_> {
    fn from(value: i32) -> Self {
        Self::Int32(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int64(value)
    }
}

impl From<i64> for ValueRef<'_> {
    fn from(value: i64) -> Self {
        Self::Int64(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float32(value)
    }
}

impl From<f32> for ValueRef<'_> {
    fn from(value: f32) -> Self {
        Self::Float32(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float64(value)
    }
}

impl From<f64> for ValueRef<'_> {
    fn from(value: f64) -> Self {
        Self::Float64(value)
    }
}

impl From<ArrayValue> for Value {
    fn from(array: ArrayValue) -> Self {
        Self::Array(Box::new(array))
    }
}

impl<'a> From<&'a ArrayValue> for ValueRef<'a> {
    fn from(value: &'a ArrayValue) -> Self {
        Self::Array(value.as_ref())
    }
}

impl From<ObjectValue> for Value {
    fn from(object: ObjectValue) -> Self {
        Self::Object(Box::new(object))
    }
}

impl<'a> From<&'a ObjectValue> for ValueRef<'a> {
    fn from(value: &'a ObjectValue) -> Self {
        Self::Object(value.as_ref())
    }
}

/// A complex number.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Complex<T: IsFloatingPoint> {
    /// The real component.
    pub real: T,

    /// The imaginary component.
    pub imag: T,
}

/// A 32-bit complex number.
pub type Complex32 = Complex<f32>;

/// A 64-bit complex number.
pub type Complex64 = Complex<f64>;

impl From<Complex32> for ObjectValue {
    fn from(Complex { real, imag }: Complex32) -> Self {
        let object = Object::new("complex32")
            .with_field("real", Type::Primitive(Primitive::Float32))
            .with_field("imag", Type::Primitive(Primitive::Float32));

        let mut data = SmallVec::new();
        data.extend_from_slice(&real.to_ne_bytes());
        data.extend_from_slice(&imag.to_ne_bytes());

        ObjectValue { ty: object, data }
    }
}

impl From<Complex32> for Value {
    fn from(value: Complex32) -> Self {
        ObjectValue::from(value).into()
    }
}

impl TryFrom<ValueRef<'_>> for Complex32 {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Object(object) => match (object.field("real"), object.field("imag")) {
                (Some(ValueRef::Float32(real)), Some(ValueRef::Float32(imag))) => {
                    Ok(Self { real, imag })
                }
                _ => Err(()),
            },
            _ => Err(()),
        }
    }
}

impl From<Complex64> for ObjectValue {
    fn from(Complex { real, imag }: Complex64) -> Self {
        let object = Object::new("complex64")
            .with_field("real", Type::Primitive(Primitive::Float64))
            .with_field("imag", Type::Primitive(Primitive::Float64));

        let mut data = SmallVec::new();
        data.extend_from_slice(&real.to_ne_bytes());
        data.extend_from_slice(&imag.to_ne_bytes());

        ObjectValue { ty: object, data }
    }
}

impl From<Complex64> for Value {
    fn from(value: Complex64) -> Self {
        ObjectValue::from(value).into()
    }
}

impl TryFrom<ValueRef<'_>> for Complex64 {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Object(object) => match (object.field("real"), object.field("imag")) {
                (Some(ValueRef::Float64(real)), Some(ValueRef::Float64(imag))) => {
                    Ok(Self { real, imag })
                }
                _ => Err(()),
            },
            _ => Err(()),
        }
    }
}

impl<T, const N: usize> From<[T; N]> for ArrayValue
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
        ArrayValue { ty: array, data }
    }
}

impl<T, const N: usize> From<[T; N]> for Value
where
    T: Into<Value> + Default,
{
    fn from(value: [T; N]) -> Self {
        ArrayValue::from(value).into()
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

impl TryFrom<ValueRef<'_>> for bool {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Bool(value) => Ok(value),
            _ => Err(()),
        }
    }
}

impl TryFrom<ValueRef<'_>> for i32 {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Int32(value) => Ok(value),
            _ => Err(()),
        }
    }
}

impl TryFrom<ValueRef<'_>> for i64 {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Int64(value) => Ok(value),
            _ => Err(()),
        }
    }
}

impl TryFrom<ValueRef<'_>> for f32 {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Float32(value) => Ok(value),
            _ => Err(()),
        }
    }
}

impl TryFrom<ValueRef<'_>> for f64 {
    type Error = ();

    fn try_from(value: ValueRef<'_>) -> Result<Self, Self::Error> {
        match value {
            ValueRef::Float64(value) => Ok(value),
            _ => Err(()),
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
        let array: Type = Array::new(Type::Primitive(Primitive::Int32), 3).into();
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
        let array: Type = Array::new(Array::new(Type::Primitive(Primitive::Int32), 3), 2).into();
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
        let ty = Object::new("test")
            .with_field("a", Type::Primitive(Primitive::Int32))
            .with_field("b", Type::Primitive(Primitive::Int64))
            .with_field(
                "c",
                Object::new("inner").with_field("d", Type::Primitive(Primitive::Bool)),
            );

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

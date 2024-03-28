//! Types of Cmajor values.

use {
    sealed::sealed,
    serde::{Deserialize, Serialize},
    smallvec::SmallVec,
    std::any::TypeId,
};

/// A Cmajor type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    /// A primitive type.
    Primitive(Primitive),

    /// An array type.
    Array(Box<Array>),

    /// An object type.
    Object(Box<Object>),
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
/// A Cmajor primitive.
pub enum Primitive {
    /// A void type.
    Void,

    /// A boolean type.
    Bool,

    /// A 32-bit signed integer type.
    Int32,

    /// A 64-bit signed integer type.
    Int64,

    /// A 32-bit floating-point type.
    Float32,

    /// A 64-bit floating-point type.
    Float64,
}

/// A reference to a Cmajor [`Type`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeRef<'a> {
    /// A primitive type.
    Primitive(Primitive),

    /// An array type.
    Array(&'a Array),

    /// An object type.
    Object(&'a Object),
}

/// An object type.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Object {
    fields: SmallVec<[Field; 2]>,
}

/// A field of an [`Object`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    name: String,
    ty: Type,
    offset: usize,
}

/// An array type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Array {
    elem_ty: Type,
    len: usize,
}

impl Type {
    /// The size of the type in bytes.
    pub fn size(&self) -> usize {
        self.as_ref().size()
    }

    /// Get a reference to the type.
    pub fn as_ref(&self) -> TypeRef<'_> {
        match self {
            Type::Primitive(primitive) => TypeRef::Primitive(*primitive),
            Type::Array(array) => TypeRef::Array(array.as_ref()),
            Type::Object(object) => TypeRef::Object(object.as_ref()),
        }
    }

    /// If the type is an object, return it.
    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Type::Object(object) => Some(object),
            _ => None,
        }
    }

    /// Returns the corresponding [`TypeId`] for the type (if any).
    pub(crate) fn type_id(&self) -> Option<TypeId> {
        match self {
            Type::Primitive(Primitive::Void) => Some(TypeId::of::<()>()),
            Type::Primitive(Primitive::Bool) => Some(TypeId::of::<bool>()),
            Type::Primitive(Primitive::Int32) => Some(TypeId::of::<i32>()),
            Type::Primitive(Primitive::Int64) => Some(TypeId::of::<i64>()),
            Type::Primitive(Primitive::Float32) => Some(TypeId::of::<f32>()),
            Type::Primitive(Primitive::Float64) => Some(TypeId::of::<f64>()),
            _ => None,
        }
    }

    /// Check whether the type is a given primitive.
    pub fn is<T>(&self) -> bool
    where
        T: IsPrimitive + 'static,
    {
        TypeId::of::<T>()
            == self
                .type_id()
                .expect("primitive types always have a type id")
    }
}

impl TypeRef<'_> {
    /// The size of the type in bytes.
    pub fn size(&self) -> usize {
        match self {
            TypeRef::Primitive(Primitive::Void) => 0,
            TypeRef::Primitive(Primitive::Bool) => 4,
            TypeRef::Primitive(Primitive::Int32) => 4,
            TypeRef::Primitive(Primitive::Int64) => 8,
            TypeRef::Primitive(Primitive::Float32) => 4,
            TypeRef::Primitive(Primitive::Float64) => 8,
            TypeRef::Array(array) => array.size(),
            TypeRef::Object(object) => object.size(),
        }
    }

    /// Convert the type reference into an owned [`Type`].
    pub fn to_owned(&self) -> Type {
        match *self {
            TypeRef::Primitive(primitive) => Type::Primitive(primitive),
            TypeRef::Array(array) => Type::Array(Box::new(array.clone())),
            TypeRef::Object(object) => Type::Object(Box::new(object.clone())),
        }
    }
}

impl Array {
    /// Create a new array type.
    pub fn new(elem_ty: impl Into<Type>, len: usize) -> Self {
        Array {
            elem_ty: elem_ty.into(),
            len,
        }
    }

    /// The size of the array in bytes.
    pub fn size(&self) -> usize {
        self.elem_ty.size() * self.len
    }

    /// The type of the array's elements.
    pub fn elem_ty(&self) -> &Type {
        &self.elem_ty
    }

    /// The number of elements in the array.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the array is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Object {
    /// Create a new object type.
    pub fn new() -> Self {
        Object::default()
    }

    /// The size of the object in bytes.
    pub fn size(&self) -> usize {
        self.fields.iter().map(|field| field.ty.size()).sum()
    }

    /// Add a [`Field`] to the object.
    pub fn add_field(&mut self, name: impl AsRef<str>, ty: impl Into<Type>) {
        let size = self.size();
        self.fields.push(Field {
            name: name.as_ref().to_owned(),
            ty: ty.into(),
            offset: size,
        });
    }

    /// Add a [`Field`] to the object.
    pub fn with_field(mut self, name: impl AsRef<str>, ty: impl Into<Type>) -> Self {
        self.add_field(name, ty.into());
        self
    }

    /// The fields of the object.
    pub fn fields(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter()
    }
}

impl From<Primitive> for Type {
    fn from(primitive: Primitive) -> Self {
        Type::Primitive(primitive)
    }
}

impl From<Array> for Type {
    fn from(array: Array) -> Self {
        Type::Array(Box::new(array))
    }
}

impl From<Object> for Type {
    fn from(object: Object) -> Self {
        Type::Object(Box::new(object))
    }
}

impl Field {
    /// The name of the field.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The type of the field.
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// The offset of the field in the object.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

/// Implemented for primitive types.
#[sealed]
pub trait IsPrimitive {}

macro_rules! impl_is_primitive {
    ($($ty:ty),*) => {
        $(
            #[sealed]
            impl IsPrimitive for $ty {}
        )*
    };
}

impl_is_primitive!(bool, i32, i64, f32, f64);

/// Implemented for scalar types.
#[sealed]
pub trait IsScalar: IsPrimitive {}

macro_rules! impl_is_scalar {
    ($($ty:ty),*) => {
        $(
            #[sealed]
            impl IsScalar for $ty {}
        )*
    };
}

impl_is_scalar!(i32, i64, f32, f64);

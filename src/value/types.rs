//! Types of Cmajor values.

use smallvec::SmallVec;

/// The type of a Cmajor value.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
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

    /// An array type.
    Array(Box<Array>),

    /// An object type.
    Object(Box<Object>),
}

/// A reference to a Cmajor [`Type`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeRef<'a> {
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

    /// An array type.
    Array(&'a Array),

    /// An object type.
    Object(&'a Object),
}

/// An object type.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Object {
    fields: SmallVec<[Field; 2]>,
}

/// A field of an [`Object`].
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    name: String,
    ty: Type,
}

/// An array type.
#[derive(Debug, Clone, PartialEq)]
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
            Type::Void => TypeRef::Void,
            Type::Bool => TypeRef::Bool,
            Type::Int32 => TypeRef::Int32,
            Type::Int64 => TypeRef::Int64,
            Type::Float32 => TypeRef::Float32,
            Type::Float64 => TypeRef::Float64,
            Type::Array(array) => TypeRef::Array(array.as_ref()),
            Type::Object(object) => TypeRef::Object(object.as_ref()),
        }
    }
}

impl TypeRef<'_> {
    /// The size of the type in bytes.
    pub fn size(&self) -> usize {
        match self {
            TypeRef::Void => 0,
            TypeRef::Bool => 4,
            TypeRef::Int32 => 4,
            TypeRef::Int64 => 8,
            TypeRef::Float32 => 4,
            TypeRef::Float64 => 8,
            TypeRef::Array(array) => array.size(),
            TypeRef::Object(object) => object.size(),
        }
    }

    /// Convert the type reference into an owned [`Type`].
    pub fn to_owned(&self) -> Type {
        match *self {
            TypeRef::Void => Type::Void,
            TypeRef::Bool => Type::Bool,
            TypeRef::Int32 => Type::Int32,
            TypeRef::Int64 => Type::Int64,
            TypeRef::Float32 => Type::Float32,
            TypeRef::Float64 => Type::Float64,
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
        self.fields.push(Field {
            name: name.as_ref().to_owned(),
            ty: ty.into(),
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
}

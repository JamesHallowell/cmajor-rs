#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    Bool,
    Int32,
    Int64,
    Float32,
    Float64,
    Array(Box<Array>),
    Object(Box<Object>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    name: String,
    ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Array {
    ty: Type,
    len: usize,
}

impl Type {
    pub fn size(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Bool => 4,
            Type::Int32 => 4,
            Type::Int64 => 8,
            Type::Float32 => 4,
            Type::Float64 => 8,
            Type::Array(ref array) => array.ty.size() * array.len,
            Type::Object(ref object) => object.fields.iter().map(|field| field.ty.size()).sum(),
        }
    }
}

impl Array {
    pub fn new(ty: impl Into<Type>, len: usize) -> Self {
        Array { ty: ty.into(), len }
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl Object {
    pub fn new() -> Self {
        Object { fields: Vec::new() }
    }

    pub fn add_field(&mut self, name: impl AsRef<str>, ty: impl Into<Type>) {
        self.fields.push(Field {
            name: name.as_ref().to_owned(),
            ty: ty.into(),
        });
    }

    pub fn with_field(mut self, name: impl AsRef<str>, ty: impl Into<Type>) -> Self {
        self.add_field(name, ty.into());
        self
    }

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
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }
}

pub trait IsType: sealed::Sealed {
    fn get_type() -> Type;
}

impl IsType for bool {
    fn get_type() -> Type {
        Type::Bool
    }
}

impl IsType for i32 {
    fn get_type() -> Type {
        Type::Int32
    }
}

impl IsType for i64 {
    fn get_type() -> Type {
        Type::Int64
    }
}

impl IsType for f32 {
    fn get_type() -> Type {
        Type::Float32
    }
}

impl IsType for f64 {
    fn get_type() -> Type {
        Type::Float64
    }
}

mod sealed {
    pub trait Sealed {}

    impl Sealed for bool {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

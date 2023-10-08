use smallvec::SmallVec;

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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeRef<'a> {
    Void,
    Bool,
    Int32,
    Int64,
    Float32,
    Float64,
    Array(&'a Array),
    Object(&'a Object),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Object {
    fields: SmallVec<[Field; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    name: String,
    ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Array {
    elem_ty: Type,
    len: usize,
}

impl Type {
    pub fn size(&self) -> usize {
        self.as_ref().size()
    }

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
    pub fn new(elem_ty: impl Into<Type>, len: usize) -> Self {
        Array {
            elem_ty: elem_ty.into(),
            len,
        }
    }

    pub fn size(&self) -> usize {
        self.elem_ty.size() * self.len
    }

    pub fn elem_ty(&self) -> &Type {
        &self.elem_ty
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Object {
    pub fn new() -> Self {
        Object::default()
    }

    pub fn size(&self) -> usize {
        self.fields.iter().map(|field| field.ty.size()).sum()
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

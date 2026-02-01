use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Declaration {
    Schema(Schema),
    Flow(Flow),
    Import(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Option<Type>,
    pub implementation: Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Implementation {
    Linked(String), // Implementation in a separate file (ais)
    Inline(Vec<String>), // Intent lines or manual implementation lines
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Type {
    I32,
    F32,
    Bool,
    Str,
    Void,
    Custom(String),
    Stream(Box<Type>),
    List(Box<Type>),
    Option(Box<Type>),
}

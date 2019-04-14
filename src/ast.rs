use std::fmt::Debug;

pub trait AST: Debug {}

// Expression
#[derive(Debug)]
pub enum Expr {
    NumberExpr(NumberExpr),
    VariableExpr(VariableExpr),
    BinaryExpr(BinaryExpr),
    CallExpr(CallExpr),
}

// Number
#[derive(Debug)]
pub struct NumberExpr {
    pub val: f64,
}

// Variable
#[derive(Debug)]
pub struct VariableExpr {
    pub name: String,
}

// Binary operation
#[derive(Debug)]
pub struct BinaryExpr {
    pub op: char,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
}

// Function call
#[derive(Debug)]
pub struct CallExpr {
    pub callee: String,
    pub args: Vec<Box<Expr>>,
}

impl AST for Expr {}

// Function prototype
#[derive(Debug)]
pub struct Prototype {
    pub name: String,
    pub args: Vec<String>,
}

impl AST for Prototype {}

// Function definition
#[derive(Debug)]
pub struct Function {
    pub proto: Box<Prototype>,
    pub body: Box<Expr>,
}

impl AST for Function {}

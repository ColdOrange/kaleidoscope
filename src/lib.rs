#[macro_use]
extern crate lazy_static;
extern crate llvm_sys as llvm;

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod jit;

use std::fmt::Debug;
use std::ffi::CString;
use std::os::raw::c_uint;
use std::ptr::null_mut;

use llvm::prelude::*;
use llvm::core::*;
use llvm::LLVMRealPredicate;
use llvm::analysis::{LLVMVerifyFunction, LLVMVerifierFailureAction};

use crate::parser::Parser;

pub trait AST: Debug {
    unsafe fn codegen(&self, parser: &mut Parser) -> LLVMValueRef;
}

// Expression
#[derive(Debug)]
pub enum Expr {
    NumberExpr(NumberExpr),
    VariableExpr(VariableExpr),
    BinaryExpr(BinaryExpr),
    CallExpr(CallExpr),
}

impl AST for Expr {
    unsafe fn codegen(&self, parser: &mut Parser) -> LLVMValueRef {
        match self {
            Expr::NumberExpr(n) => n.codegen(parser),
            Expr::VariableExpr(v) => v.codegen(parser),
            Expr::BinaryExpr(b) => b.codegen(parser),
            Expr::CallExpr(c) => c.codegen(parser),
        }
    }
}

// Number
#[derive(Debug)]
pub struct NumberExpr {
    pub val: f64,
}

impl AST for NumberExpr {
    unsafe fn codegen(&self, parser: &mut Parser) -> LLVMValueRef {
        LLVMConstReal(parser.get_double_type(), self.val)
    }
}

// Variable
#[derive(Debug)]
pub struct VariableExpr {
    pub name: String,
}

impl AST for VariableExpr {
    unsafe fn codegen(&self, parser: &mut Parser) -> LLVMValueRef {
        match parser.get_named_value(self.name.clone()) {
            Some(value) => *value,
            None => panic!("unknown variable name <{}>", self.name)
        }
    }
}

// Binary operation
#[derive(Debug)]
pub struct BinaryExpr {
    pub op: char,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
}

impl AST for BinaryExpr {
    unsafe fn codegen(&self, parser: &mut Parser) -> LLVMValueRef {
        let lhs = self.lhs.codegen(parser);
        let rhs = self.rhs.codegen(parser);
        match self.op {
            '+' => LLVMBuildFAdd(parser.builder(), lhs, rhs, CString::new("addtmp").unwrap().into_raw()),
            '-' => LLVMBuildFSub(parser.builder(), lhs, rhs, CString::new("subtmp").unwrap().into_raw()),
            '*' => LLVMBuildFMul(parser.builder(), lhs, rhs, CString::new("multmp").unwrap().into_raw()),
            '<' => {
                let cmp_value = LLVMBuildFCmp(parser.builder(), LLVMRealPredicate::LLVMRealULT, lhs, rhs, CString::new("cmptmp").unwrap().into_raw());
                LLVMBuildUIToFP(parser.builder(), cmp_value, parser.get_double_type(), CString::new("booltmp").unwrap().into_raw())
            }
            _ => panic!("invalid binary operation <{}>", self.op)
        }
    }
}

// Function call
#[derive(Debug)]
pub struct CallExpr {
    pub callee: String,
    pub args: Vec<Box<Expr>>,
}

impl AST for CallExpr {
    unsafe fn codegen(&self, parser: &mut Parser) -> LLVMValueRef {
        let function = LLVMGetNamedFunction(parser.module(), CString::new(self.callee.clone()).unwrap().into_raw());
        if function == null_mut() {
            panic!("unknown function name <{}>", self.callee);
        }

        if LLVMCountParams(function) != self.args.len() as u32 {
            panic!("invalid param number, expected {}, got {}", LLVMCountParams(function), self.args.len());
        }

        let mut args = Vec::new();
        for arg in self.args.iter() {
            args.push(arg.codegen(parser))
        }

        LLVMBuildCall(parser.builder(), function, args.as_mut_ptr(), args.len() as c_uint, CString::new("calltmp").unwrap().into_raw())
    }
}

// Function prototype
#[derive(Debug)]
pub struct Prototype {
    pub name: String,
    pub args: Vec<String>,
}

impl AST for Prototype {
    unsafe fn codegen(&self, parser: &mut Parser) -> LLVMValueRef {
        let function_type = parser.get_function_type(self.args.len());
        let function = LLVMAddFunction(parser.module(), CString::new(self.name.clone()).unwrap().into_raw(), function_type);
//        if LLVMCountBasicBlocks(function) != 0 {
//            panic!("redefinition of function");
//        }

        for (i, arg) in self.args.iter().enumerate() {
            let function_arg = LLVMGetParam(function, i as c_uint);
            LLVMSetValueName2(function_arg, CString::new(arg.clone()).unwrap().into_raw(), arg.len());
            parser.insert_named_value(arg.clone(), function_arg);
        }

        function
    }
}

// Function definition
#[derive(Debug)]
pub struct Function {
    pub proto: Box<Prototype>,
    pub body: Box<Expr>,
}

impl AST for Function {
    unsafe fn codegen(&self, parser: &mut Parser) -> LLVMValueRef {
        parser.clear_named_value();

        let function = self.proto.codegen(parser);
        let basic_block = LLVMAppendBasicBlockInContext(parser.context(), function, CString::new("entry").unwrap().into_raw());
        LLVMPositionBuilderAtEnd(parser.builder(), basic_block);
        let body = self.body.codegen(parser);
        LLVMBuildRet(parser.builder(), body);

        if LLVMVerifyFunction(function, LLVMVerifierFailureAction::LLVMPrintMessageAction) != 0 {
            panic!("function verify failed");
        }

        LLVMRunFunctionPassManager(parser.function_pass_manager(), function);
        function
    }
}

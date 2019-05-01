use std::collections::HashMap;
use std::ffi::{CString, CStr};
use std::os::raw::c_uint;

use llvm::prelude::*;
use llvm::core::*;
use llvm::target::*;
use llvm::transforms::scalar::*;

use crate::lexer::{Lexer, Token};
use crate::ast::{AST, Expr, NumberExpr, VariableExpr, BinaryExpr, CallExpr, Prototype, Function};

pub struct Parser<'b> {
    lexer: Lexer<'b>,
    token: Option<Token>,
    ast: Vec<Box<AST>>,
    codegen: Vec<String>,
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    module: LLVMModuleRef,
    name_values: HashMap<String, LLVMValueRef>,
    function_pass_manager: LLVMPassManagerRef,
}

impl<'b> Parser<'b> {
    pub fn new(buf: &'b str) -> Parser<'b> {
        unsafe {
            if LLVM_InitializeNativeTarget() != 0 {
                panic!("initialize native target failed");
            }
            if LLVM_InitializeNativeAsmPrinter() != 0 {
                panic!("initialize native asm printer failed");
            }
            if LLVM_InitializeNativeAsmParser() != 0 {
                panic!("initialize native asm parser failed");
            }
        }

        let context = unsafe {
            LLVMContextCreate()
        };
        let builder = unsafe {
            LLVMCreateBuilderInContext(context)
        };
        let module = unsafe {
            LLVMModuleCreateWithNameInContext(CString::new("kaleidoscope").unwrap().into_raw(), context)
        };
        let function_pass_manager = unsafe {
            LLVMCreateFunctionPassManagerForModule(module)
        };
        unsafe {
            // optimization passes
            LLVMAddBasicAliasAnalysisPass(function_pass_manager);
            LLVMAddInstructionCombiningPass(function_pass_manager);
            LLVMAddReassociatePass(function_pass_manager);
            LLVMAddGVNPass(function_pass_manager);
            LLVMAddCFGSimplificationPass(function_pass_manager);

            LLVMInitializeFunctionPassManager(function_pass_manager);
        }

        Parser {
            lexer: Lexer::new(buf),
            token: None,
            ast: Vec::new(),
            codegen: Vec::new(),
            context: context,
            builder: builder,
            module: module,
            name_values: HashMap::new(),
            function_pass_manager: function_pass_manager,
        }
    }

    #[inline]
    pub fn token(&self) -> Option<Token> { self.token.clone() }

    #[inline]
    pub fn context(&self) -> LLVMContextRef { self.context }

    #[inline]
    pub fn builder(&self) -> LLVMBuilderRef { self.builder }

    #[inline]
    pub fn module(&self) -> LLVMModuleRef { self.module }

    #[inline]
    pub fn function_pass_manager(&self) -> LLVMPassManagerRef { self.function_pass_manager }

    #[inline]
    pub fn get_named_value(&self, name: String) -> Option<&LLVMValueRef> {
        self.name_values.get(&name)
    }

    #[inline]
    pub fn insert_named_value(&mut self, name: String, value: LLVMValueRef) -> Option<LLVMValueRef> {
        self.name_values.insert(name, value)
    }

    #[inline]
    pub fn clear_named_value(&mut self) {
        self.name_values.clear()
    }

    #[inline]
    pub fn get_double_type(&self) -> LLVMTypeRef {
        unsafe { LLVMDoubleTypeInContext(self.context) }
    }

    #[inline]
    pub fn get_function_type(&self, argc: usize) -> LLVMTypeRef {
        let mut arg_types = vec![self.get_double_type(); argc];
        unsafe { LLVMFunctionType(self.get_double_type(), arg_types.as_mut_ptr(), argc as c_uint, 0) }
    }

    #[inline]
    fn get_codegen_string<T: AST>(&mut self, ast: &Box<T>) -> String {
        unsafe {
            let codegen = ast.codegen(self);
            CStr::from_ptr(LLVMPrintValueToString(codegen)).to_str().unwrap().to_owned()
        }
    }

    #[inline]
    pub fn get_next_token(&mut self) {
        self.token = self.lexer.next();
    }

    // top ::= definition | extern | expression | ';'
    pub fn parse(&mut self) {
        loop {
            self.get_next_token();

            match self.token {
                None => break,
                Some(Token::Def) => {
                    let def = self.parse_definition();
                    let codegen = self.get_codegen_string(&def);
                    self.ast.push(def);
                    self.codegen.push(codegen);
                }
                Some(Token::Extern) => {
                    let ext = self.parse_extern();
                    let codegen = self.get_codegen_string(&ext);
                    self.ast.push(ext);
                    self.codegen.push(codegen);
                }
                Some(Token::Symbol(';')) => continue,
                _ => {
                    let exp = self.parse_expression();
                    let codegen = self.get_codegen_string(&exp);
                    self.ast.push(exp);
                    self.codegen.push(codegen);
                }
            }
        }
    }

    // definition ::= 'def' prototype expression
    pub fn parse_definition(&mut self) -> Box<Function> {
        assert_eq!(self.token, Some(Token::Def));
        self.get_next_token();

        Box::new(Function {
            proto: self.parse_prototype(),
            body: self.parse_expression(),
        })
    }

    // prototype ::= id '(' id* ')'
    fn parse_prototype(&mut self) -> Box<Prototype> {
        let name = match self.token.clone() {
            Some(Token::Identifier(id)) => id,
            _ => panic!("unexpected token: expected Identifier, got {:?}", self.token)
        };
        self.get_next_token();

        assert_eq!(self.token, Some(Token::Symbol('(')));
        self.get_next_token();

        let mut args = Vec::new();
        loop {
            match self.token.clone() {
                Some(Token::Identifier(id)) => {
                    args.push(id);
                    self.get_next_token();
                }
                Some(Token::Symbol(')')) => {
                    self.get_next_token();
                    break;
                }
                _ => panic!("unexpected token: expected ')', got {:?}", self.token)
            }
        }
        Box::new(Prototype {
            name: name,
            args: args,
        })
    }

    // extern ::= 'extern' prototype
    pub fn parse_extern(&mut self) -> Box<Prototype> {
        assert_eq!(self.token, Some(Token::Extern));
        self.get_next_token();

        self.parse_prototype()
    }

    // expression ::= primary binoprhs
    pub fn parse_expression(&mut self) -> Box<Expr> {
        let lhs = self.parse_primary();
        self.parse_binoprhs(lhs, 0)
    }

    // primary ::= id ['(' expression* ')'] | number | '(' expression ')'
    fn parse_primary(&mut self) -> Box<Expr> {
        match self.token.clone() {
            Some(Token::Identifier(id)) => {
                let name = id;
                self.get_next_token();

                if self.token == Some(Token::Symbol('(')) {
                    self.get_next_token();

                    let mut args = Vec::new();
                    loop {
                        match self.token {
                            Some(Token::Symbol(')')) => {
                                self.get_next_token();
                                break;
                            }
                            Some(Token::Symbol(',')) => {
                                self.get_next_token();
                            }
                            _ => {
                                args.push(self.parse_expression())
                            }
                        }
                    }
                    Box::new(Expr::CallExpr(CallExpr {
                        callee: name,
                        args: args,
                    }))
                } else {
                    Box::new(Expr::VariableExpr(VariableExpr { name: name }))
                }
            }
            Some(Token::Number(n)) => {
                self.get_next_token();
                Box::new(Expr::NumberExpr(NumberExpr { val: n }))
            }
            Some(Token::Symbol('(')) => {
                self.get_next_token();
                let expr = self.parse_expression();

                if self.token == Some(Token::Symbol(')')) {
                    self.get_next_token();
                    expr
                } else {
                    panic!("unexpected token: expected ')', got {:?}", self.token)
                }
            }
            _ => panic!("unexpected token: expected [ id | number | '(' ], got {:?}", self.token)
        }
    }

    // binoprhs ::= ('+' primary)*
    fn parse_binoprhs(&mut self, mut lhs: Box<Expr>, lhs_precedence: i32) -> Box<Expr> {
        loop {
            let precedence = self.get_token_precedence();
            if precedence.1 < lhs_precedence {
                return lhs;
            }

            self.get_next_token();
            let mut rhs = self.parse_primary();

            // If BinOp binds less tightly with RHS than the operator after RHS,
            // let the pending operator take RHS as its LHS.
            let next_precedence = self.get_token_precedence();
            if precedence.1 < next_precedence.1 {
                rhs = self.parse_binoprhs(rhs, precedence.1 + 1);
            }

            lhs = Box::new(Expr::BinaryExpr(BinaryExpr {
                op: precedence.0,
                lhs: lhs,
                rhs: rhs,
            }))
        }
    }

    fn get_token_precedence(&self) -> (char, i32) {
        match self.token {
            Some(Token::Symbol(op)) if BINOP_PRECEDENCE.contains_key(&op) => {
                (op, *BINOP_PRECEDENCE.get(&op).unwrap())
            }
            _ => (' ', -1)
        }
    }
}

lazy_static! {
    static ref BINOP_PRECEDENCE: HashMap<char, i32> = {
        let mut m = HashMap::new();
        m.insert('<', 10);
        m.insert('+', 20);
        m.insert('-', 20);
        m.insert('*', 40); // highest
        m
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let mut parser = Parser::new(r"
# An incomplete (and wrong) example, because if-stat is not supported for now
def fib(x)
    fib(x-1)+fib(x-2)
");

        // TODO: can't use PartialEq on trait object
        parser.parse();
        println!("{:#?}", parser.ast);
    }

    #[test]
    fn test_codegen() {
        let mut parser = Parser::new(r"
# An incomplete (and wrong) example, because if-stat is not supported for now
def fib(x)
    fib(x-1)+fib(x-2)
");

        parser.parse();
        parser.codegen.iter().for_each(|c| println!("{}", c));
    }
}

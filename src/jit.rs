use std::io::{self, Write};
use std::ffi::CString;

use llvm::core::LLVMDumpValue;
use llvm::execution_engine::*;

use crate::lexer::Token;
use crate::parser::Parser;
use crate::ast::{AST, Function, Prototype};

pub struct JIT<'b> {
    parser: Parser<'b>,
    execution_engine: LLVMExecutionEngineRef,
}

impl<'b> JIT<'b> {
    pub fn new(buf: &'b str) -> JIT<'b> {
        let parser = Parser::new(buf);
        let execution_engine = unsafe {
            LLVMLinkInMCJIT();
            let mut execution_engine: LLVMExecutionEngineRef = 0 as LLVMExecutionEngineRef;
            let mut error: *mut i8 = 0 as *mut i8;
            if LLVMCreateExecutionEngineForModule(&mut execution_engine, parser.module(), &mut error) != 0 {
                panic!("create execution engine failed: {}", CString::from_raw(error).into_string().unwrap());
            }
            execution_engine
        };

        JIT {
            parser: parser,
            execution_engine: execution_engine,
        }
    }

    pub fn run(&mut self) {
        loop {
            print!("ready> ");
            io::stdout().flush().unwrap();
            self.parser.get_next_token();

            match self.parser.token() {
                None => break,
                Some(Token::Def) => {
                    let def = self.parser.parse_definition();
                    println!("Parsed a definition");
                    unsafe {
                        LLVMDumpValue(def.codegen(&mut self.parser));
                    }
                }
                Some(Token::Extern) => {
                    let ext = self.parser.parse_extern();
                    println!("Parsed an extern");
                    unsafe {
                        LLVMDumpValue(ext.codegen(&mut self.parser));
                    }
                }
                Some(Token::Symbol(';')) => continue,
                _ => {
                    let exp = self.parser.parse_expression();
                    unsafe {
                        let anonymous_function = Function {
                            proto: Box::new(Prototype { name: "".to_string(), args: vec![] }),
                            body: exp,
                        };
                        let mut args: Vec<LLVMGenericValueRef> = Vec::new();
                        let ret = LLVMRunFunction(self.execution_engine,
                                                  anonymous_function.codegen(&mut self.parser),
                                                  0, args.as_mut_ptr());
                        let double_ret = LLVMGenericValueToFloat(self.parser.get_double_type(), ret);
                        println!("Returned {}", double_ret);
                    };
                }
            }
        }
    }
}

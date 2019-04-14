use std::collections::HashMap;

use crate::lexer::{Lexer, Token};
use crate::ast::{AST, Expr, NumberExpr, VariableExpr, BinaryExpr, CallExpr, Prototype, Function};

pub struct Parser<'b> {
    lexer: Lexer<'b>,
    token: Option<Token>,
    ast: Vec<Box<AST>>,
}

impl<'b> Parser<'b> {
    #[inline]
    pub fn new(buf: &'b str) -> Parser<'b> {
        Parser {
            lexer: Lexer::new(buf),
            token: None,
            ast: Vec::new(),
        }
    }

    #[inline]
    fn get_next_token(&mut self) {
        self.token = self.lexer.next();
    }

    // top ::= definition | extern | expression | ';'
    pub fn parse(&mut self) -> &Vec<Box<AST>> {
        loop {
            self.get_next_token();

            match self.token {
                None => break,
                Some(Token::Def) => {
                    let def = self.parse_definition();
                    self.ast.push(def);
                }
                Some(Token::Extern) => {
                    let ext = self.parse_extern();
                    self.ast.push(ext);
                }
                Some(Token::Symbol(';')) => continue,
                _ => {
                    let exp = self.parse_expression();
                    self.ast.push(exp);
                }
            }
        }

        &self.ast
    }

    // definition ::= 'def' prototype expression
    fn parse_definition(&mut self) -> Box<Function> {
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
    fn parse_extern(&mut self) -> Box<Prototype> {
        assert_eq!(self.token, Some(Token::Extern));
        self.get_next_token();

        self.parse_prototype()
    }

    // expression ::= primary binoprhs
    fn parse_expression(&mut self) -> Box<Expr> {
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
                self.get_next_token();

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
    fn test_parser() {
        let mut parser = Parser::new(r"
# An incomplete (and wrong) example, because if-stat is not supported for now
def fib(x)
    fib(x-1)+fib(x-2)
");

        println!("{:#?}", parser.parse());
    }
}

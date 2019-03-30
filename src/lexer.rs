use std::str;
use core::slice;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    // keywords
    Def,
    Extern,
    // primary
    Identifier(String),
    Number(f64),
    // symbol
    Symbol(char),
}

pub struct Lexer<'b> {
    buf: &'b str,
    pos: usize,
}

impl<'b> Lexer<'b> {
    #[inline]
    fn new(buf: &'b str) -> Lexer<'b> {
        Lexer {
            buf: buf,
            pos: 0,
        }
    }

    fn peek(&mut self) -> Option<char> {
        if self.pos < self.buf.len() {
            let b = unsafe { *self.buf.as_bytes().get_unchecked(self.pos) };
            Some(b as char)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.buf.len() {
            let b = unsafe { *self.buf.as_bytes().get_unchecked(self.pos) };
            if !b.is_ascii_whitespace() {
                break;
            }
            self.pos += 1;
        }
    }

    fn skip_line(&mut self) {
        while self.pos < self.buf.len() {
            let b = unsafe { *self.buf.as_bytes().get_unchecked(self.pos) };
            self.pos += 1;
            if b == b'\n' {
                break;
            }
        }
    }

    fn number(&mut self) -> &'b str {
        let start = self.pos;
        while self.pos < self.buf.len() {
            let b = unsafe { *self.buf.as_bytes().get_unchecked(self.pos) };
            if !b.is_ascii_digit() && b != b'.' {
                break;
            }
            self.pos += 1;
        }
        unsafe {
            let slice = slice::from_raw_parts(self.buf.as_ptr().offset(start as isize), self.pos - start);
            str::from_utf8_unchecked(slice)
        }
    }

    fn identifier(&mut self) -> &'b str {
        let start = self.pos;
        while self.pos < self.buf.len() {
            let b = unsafe { *self.buf.as_bytes().get_unchecked(self.pos) };
            if !b.is_ascii_alphanumeric() {
                break;
            }
            self.pos += 1;
        }
        unsafe {
            let slice = slice::from_raw_parts(self.buf.as_ptr().offset(start as isize), self.pos - start);
            str::from_utf8_unchecked(slice)
        }
    }
}

impl<'b> Iterator for Lexer<'b> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        self.skip_whitespace();

        match self.peek() {
            // eof
            None => None,
            // comment
            Some('#') => {
                self.skip_line();
                self.next()
            },
            // identifier
            Some(c) if c.is_alphabetic() => {
                let i = self.identifier();
                if KEYWORDS.contains_key(i) {
                    Some(KEYWORDS.get(i).unwrap().clone())
                } else {
                    Some(Token::Identifier(i.to_string()))
                }
            },
            // number
            Some(c) if c.is_ascii_digit() || c == '.' => {
                let n = self.number().parse::<f64>().unwrap();
                Some(Token::Number(n))
            },
            // symbol
            _ => {
                let s = unsafe { *self.buf.as_bytes().get_unchecked(self.pos) as char };
                self.pos += 1;
                Some(Token::Symbol(s))
            }
        }
    }
}

lazy_static! {
    static ref KEYWORDS: HashMap<&'static str, Token> = {
        let mut m = HashMap::new();
        m.insert("def", Token::Def);
        m.insert("extern", Token::Extern);
        m
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let mut lexer = Lexer::new(r"
# Compute the x'th fibonacci number.
def fib(x)
  if x < 3 then
    1
  else
    fib(x-1)+fib(x-2)

# This expression will compute the 40th number.
fib(40)
");

        assert_eq!(lexer.next().unwrap(), Token::Def);
        assert_eq!(lexer.next().unwrap(), Token::Identifier("fib".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Symbol('('));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("x".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Symbol(')'));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("if".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("x".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Symbol('<'));
        assert_eq!(lexer.next().unwrap(), Token::Number(3.0));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("then".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Number(1.0));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("else".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("fib".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Symbol('('));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("x".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Symbol('-'));
        assert_eq!(lexer.next().unwrap(), Token::Number(1.0));
        assert_eq!(lexer.next().unwrap(), Token::Symbol(')'));
        assert_eq!(lexer.next().unwrap(), Token::Symbol('+'));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("fib".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Symbol('('));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("x".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Symbol('-'));
        assert_eq!(lexer.next().unwrap(), Token::Number(2.0));
        assert_eq!(lexer.next().unwrap(), Token::Symbol(')'));
        assert_eq!(lexer.next().unwrap(), Token::Identifier("fib".to_string()));
        assert_eq!(lexer.next().unwrap(), Token::Symbol('('));
        assert_eq!(lexer.next().unwrap(), Token::Number(40.0));
        assert_eq!(lexer.next().unwrap(), Token::Symbol(')'));
        assert!(lexer.next().is_none());
    }
}

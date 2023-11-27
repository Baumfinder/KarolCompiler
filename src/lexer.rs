use std::vec;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenType {
    Identifier,
    Equals,
    Number,
    Parenthesis,
    Operator,
    Keyword,
    Newline,
    Comma,
    EOF,
}

#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub line: usize,
    pub column: usize,
    pub file: String,
}

impl SourceInfo {
    pub fn print(&self) -> String { return format!("(\"{}\": {}, {})", self.file, self.line, self.column) }

    pub fn new(l: usize, c: usize, f: String) -> SourceInfo {
        return SourceInfo { line: l, column: c, file: f };
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub ttype: TokenType,
    pub value: String,
    pub info: SourceInfo,
}

impl Token {
    pub fn new(t: TokenType, v: String, s: SourceInfo) -> Token {
        return Token{ttype: t, value: v, info: s};
    }
    pub fn from_type(t: TokenType, s: SourceInfo) -> Token {
        return Token{ttype: t, value: String::new(), info: s};
    }

    pub fn equals(&self, t: TokenType, v: &str) -> bool {
        return self.ttype == t && self.value == v.to_string();
    }

    pub fn position(&self) -> String { return self.info.print() }
}

pub fn lexer(input_string: String) -> Vec<Token> {
    let mut input: Vec<char> = input_string.chars().collect();
    let filename: String = "file".to_string();
    input.push(' ');
    let mut tokenlist: Vec<Token> = Vec::new();

    let mut linectr: usize = 1;
    let mut colctr: usize = 1;

    let mut i: usize = 0;
    while i < input.len()
    {
        // Identifier / Keyword
        if input[i].is_ascii_alphabetic() {
            let mut tmp = String::new();
            tmp.push(input[i]);
            while input[i+1].is_ascii_alphabetic() {
                i += 1;
                tmp.push(input[i]);
            }
            if vec!["var", "arr", "func", "if", "while", "return", "call", "deref", "addr"].contains(&tmp.as_str()) {
                tokenlist.push(Token::new(TokenType::Keyword, tmp, SourceInfo::new(linectr, colctr, filename.clone())));
            } else {
                tokenlist.push(Token::new(TokenType::Identifier, tmp, SourceInfo::new(linectr, colctr, filename.clone())));
            }

            i += 1;
            continue;
        }

        // Number
        if input[i].is_digit(10) {
            let mut tmp: isize = input[i].to_digit(10).unwrap() as isize;
            while input[i+1].is_digit(10) {
                i += 1;
                tmp *= 10;
                tmp += input[i].to_digit(10).unwrap() as isize;
            }
            tokenlist.push(Token::new(TokenType::Number, format!("{}", tmp), SourceInfo::new(linectr, colctr, filename.clone())));

            i += 1;
            continue;
        }

        // Single letter operators
        if "+-*<>".contains(input[i]) {
            tokenlist.push(Token::new(TokenType::Operator, String::from(input[i]), SourceInfo::new(linectr, colctr, filename.clone())));

            i += 1;
            continue;
        }

        // Multi letter operators
        if "=<>!".contains(input[i]) && input[i+1] == '=' {
            tokenlist.push(Token::new(TokenType::Operator, format!("{}=", input[i]), SourceInfo::new(linectr, colctr, filename.clone())));
            i += 2;
            continue;
        }

        //Equals
        if input[i] == '=' {
            tokenlist.push(Token::from_type(TokenType::Equals, SourceInfo::new(linectr, colctr, filename.clone())));
            i += 1;
            continue;
        }

        //Parenthesis
        if "()[]{}".contains(input[i]) {
            tokenlist.push(Token::new(TokenType::Parenthesis, String::from(input[i]), SourceInfo::new(linectr, colctr, filename.clone())));
            i += 1;
            continue;
        }

        //Seperator
        if input[i] == ',' {
            tokenlist.push(Token::from_type(TokenType::Comma, SourceInfo::new(linectr, colctr, filename.clone())));
            i += 1;
            continue;
        }

        //Newline
        if input[i] == '\n' {
            tokenlist.push(Token::from_type(TokenType::Newline, SourceInfo::new(linectr, colctr, filename.clone())));
            linectr += 1;
            colctr = 1;

            i += 1;

            // Comments
            if input[i] == '/' && input[i+1] == '/' {
                while input[i] != '\n' {
                    i += 1;
                }
                i += 1;
            }
            continue;
        }

        i += 1;
        colctr += 1;
    }

    tokenlist.push(Token::from_type(TokenType::EOF, SourceInfo::new(linectr, colctr, filename.clone())));
    return tokenlist;
}
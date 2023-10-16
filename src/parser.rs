pub mod expr_parser;
pub mod for_loop_parser;
use crate::{
    parser::expr_parser::generate_expr_ast,
    tokenizer::{Token, VarType},
};
use expr_parser::{BinOpPrecedenceLevel, Expr};
use for_loop_parser::generate_for_loop_ast;

#[derive(Debug)]
pub struct Program {
    pub function: Function,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Continue,
    Break,
    Return(Expr),
    Declare(String, Option<Expr>, VarType),
    CompoundStmt(Vec<Statement>),
    If(Expr, Box<Statement>, Option<Box<Statement>>),
    While(Expr, Box<Statement>),
    For(Box<Statement>, Option<Expr>, Option<Expr>, Box<Statement>),
    Expr(Expr),
    Empty,
}

pub struct TokenCursor {
    contents: Vec<Token>,
    index: usize,
}

impl TokenCursor {
    pub fn new(contents: Vec<Token>) -> Self {
        TokenCursor { contents, index: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.contents.get(self.index)
    }
    fn peek_nth(&self, n: usize) -> Option<&Token> {
        // peek_nth(1) is equivalent to peek()
        self.contents.get(self.index + n - 1)
    }

    fn next(&mut self) -> Option<&Token> {
        self.index += 1;
        self.contents.get(self.index - 1)
    }
}

pub fn generate_program_ast(tokens: Vec<Token>) -> Program {
    let mut tokens = TokenCursor::new(tokens);
    let f = generate_function_ast(&mut tokens);
    assert_eq!(tokens.next(), None);
    Program { function: f }
}

fn generate_function_ast(tokens: &mut TokenCursor) -> Function {
    let function_name;

    match tokens.next() {
        Some(&Token::Type(..)) => {
            // ok
        }
        _ => {
            panic!("function definitions must begin with the type that they return!")
        }
    }

    if let Some(Token::Identifier { val }) = tokens.next() {
        function_name = val.clone();
    } else {
        panic!();
    }

    assert_eq!(tokens.next(), Some(&Token::OpenParen));
    assert_eq!(tokens.next(), Some(&Token::CloseParen));

    let body = generate_compound_stmt_ast(tokens);

    Function {
        name: function_name,
        body,
    }
}

fn generate_compound_stmt_ast(tokens: &mut TokenCursor) -> Vec<Statement> {
    assert_eq!(tokens.next(), Some(&Token::OpenBrace));
    let mut statements = Vec::new();

    while tokens.peek().is_some() && *tokens.peek().unwrap() != Token::CloseBrace {
        statements.push(generate_statement_ast(tokens));
    }

    assert_eq!(tokens.next(), Some(&Token::CloseBrace));
    return statements;
}

fn generate_statement_ast(tokens: &mut TokenCursor) -> Statement {
    let expr;

    match tokens.peek() {
        Some(Token::Continue) => {
            tokens.next();
            assert_eq!(tokens.next(), Some(&Token::Semicolon));
            return Statement::Continue;
        }
        Some(Token::Break) => {
            tokens.next();
            assert_eq!(tokens.next(), Some(&Token::Semicolon));
            return Statement::Break;
        }
        Some(Token::Return) => {
            tokens.next(); // consume the "return"

            expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());

            assert_eq!(tokens.next(), Some(&Token::Semicolon));
            return Statement::Return(expr);
        }
        Some(Token::Type(t)) => {
            let t = t.clone();
            tokens.next();
            let decl_identifier;
            let mut optional_expr = None;
            if let Some(Token::Identifier { val }) = tokens.next() {
                decl_identifier = val.clone();
            } else {
                panic!();
            }

            if tokens.peek() == Some(&Token::AssignmentEquals) {
                tokens.next();
                optional_expr = Some(generate_expr_ast(
                    tokens,
                    BinOpPrecedenceLevel::lowest_level(),
                ))
            }
            assert_eq!(tokens.next(), Some(&Token::Semicolon));
            return Statement::Declare(decl_identifier, optional_expr, t);
        }
        Some(Token::OpenBrace) => {
            let compound_stmt = generate_compound_stmt_ast(tokens);
            // note that a compound statement does not end in a semicolon, so there is no need here to consume a semicolon.
            return Statement::CompoundStmt(compound_stmt);
        }
        Some(Token::If) => {
            // consume the "if"
            tokens.next();
            assert_eq!(tokens.next(), Some(&Token::OpenParen));
            let conditional_expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());
            assert_eq!(tokens.next(), Some(&Token::CloseParen));
            let taken_branch_stmt = generate_statement_ast(tokens);
            let mut not_taken_branch_stmt = None;
            if tokens.peek() == Some(&Token::Else) {
                // consume the "else"
                tokens.next();
                not_taken_branch_stmt = Some(Box::new(generate_statement_ast(tokens)));
            }

            return Statement::If(
                conditional_expr,
                Box::new(taken_branch_stmt),
                not_taken_branch_stmt,
            );
        }
        Some(Token::While) => {
            // consume the "while"
            tokens.next();

            assert_eq!(tokens.next(), Some(&Token::OpenParen));
            let conditional = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());
            assert_eq!(tokens.next(), Some(&Token::CloseParen));

            let body = generate_statement_ast(tokens);
            return Statement::While(conditional, Box::new(body));
        }
        Some(Token::Semicolon) => {
            // consume the semicolon
            tokens.next();
            return Statement::Empty;
        }
        Some(Token::For) => {
            return generate_for_loop_ast(tokens);
        }

        _ => {
            expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());
            assert_eq!(tokens.next(), Some(&Token::Semicolon));
            return Statement::Expr(expr);
        }
    }
}

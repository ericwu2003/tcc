use super::TokenCursor;
use crate::errors::display::err_display;
use crate::tokenizer::{operator::Op, Token};

#[derive(Debug)]
pub enum Expr {
    Int(i32),
    Var(String),
    Assign(String, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    FunctionCall(String, Vec<Expr>), // Vec<Expr> contains the arguments of the function
    PostfixDec(String),
    PostfixInc(String),
    PrefixDec(String),
    PrefixInc(String),
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Negation,
    BitwiseComplement,
    Not,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BinOp {
    Multiply,
    Divide,
    Modulus,
    Plus,
    Minus,
    GreaterThan,
    GreaterThanEq,
    LessThan,
    LessThanEq,
    Equals,
    NotEquals,
    LogicalAnd,
    LogicalOr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOpPrecedenceLevel {
    MulDiv,
    AddSub,
    OrderingCmp,
    EqCmp,
    LogicalAnd,
    LogicalOr,
}

impl BinOpPrecedenceLevel {
    pub fn next_level(&self) -> Option<Self> {
        match self {
            BinOpPrecedenceLevel::LogicalOr => Some(BinOpPrecedenceLevel::LogicalAnd),
            BinOpPrecedenceLevel::LogicalAnd => Some(BinOpPrecedenceLevel::EqCmp),
            BinOpPrecedenceLevel::EqCmp => Some(BinOpPrecedenceLevel::OrderingCmp),
            BinOpPrecedenceLevel::OrderingCmp => Some(BinOpPrecedenceLevel::AddSub),
            BinOpPrecedenceLevel::AddSub => Some(BinOpPrecedenceLevel::MulDiv),
            BinOpPrecedenceLevel::MulDiv => None,
        }
    }

    pub fn lowest_level() -> Self {
        BinOpPrecedenceLevel::LogicalOr
    }
}

pub fn generate_expr_ast(
    tokens: &mut TokenCursor,
    curr_operator_precedence: BinOpPrecedenceLevel,
) -> Expr {
    if curr_operator_precedence == BinOpPrecedenceLevel::lowest_level() {
        // handle assignment of variables
        if let Some(Token::Identifier { val }) = tokens.peek() {
            let val = val.clone();
            if tokens.peek_nth(2) == Some(&Token::AssignmentEquals) {
                tokens.next();
                tokens.next();

                let rhs_expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());
                return Expr::Assign(val, Box::new(rhs_expr));
            } else if tokens.peek_nth(2) == Some(&Token::Op(Op::PlusEquals)) {
                tokens.next();
                tokens.next();

                let rhs_expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());

                return Expr::Assign(
                    val.clone(),
                    Box::new(Expr::BinOp(
                        BinOp::Plus,
                        Box::new(Expr::Var(val)),
                        Box::new(rhs_expr),
                    )),
                );
            } else if tokens.peek_nth(2) == Some(&Token::Op(Op::MinusEquals)) {
                tokens.next();
                tokens.next();

                let rhs_expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());

                return Expr::Assign(
                    val.clone(),
                    Box::new(Expr::BinOp(
                        BinOp::Minus,
                        Box::new(Expr::Var(val)),
                        Box::new(rhs_expr),
                    )),
                );
            }
        }
    }

    let mut expr: Expr;
    let next_operator_precedence_option = curr_operator_precedence.next_level();

    if let Some(next_operator_precedence) = next_operator_precedence_option {
        expr = generate_expr_ast(tokens, next_operator_precedence);
    } else {
        expr = generate_factor_ast(tokens);
    }

    while tokens.peek().is_some() {
        if &Token::QuestionMark == tokens.peek().unwrap()
            && curr_operator_precedence == BinOpPrecedenceLevel::lowest_level()
        {
            // handle ternary case. Note that ternaries have the lowest precedence level, so we need to check the precedence level.
            tokens.next();
            let first_expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());
            if tokens.next() != Some(&Token::Colon) {
                err_display(
                    format!(
                        "expected colon in ternary expression, found {:?}",
                        tokens.last().unwrap()
                    ),
                    tokens.get_last_ptr(),
                )
            }

            let second_expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());

            return Expr::Ternary(Box::new(expr), Box::new(first_expr), Box::new(second_expr));
        }

        // if the next token is a binary operator that is on the current precedence level:
        if let Some(next_op) = tokens
            .peek()
            .unwrap()
            .to_binop_precedence_level(curr_operator_precedence)
        {
            tokens.next();
            let next_expr;
            if let Some(next_operator_precedence) = next_operator_precedence_option {
                next_expr = generate_expr_ast(tokens, next_operator_precedence);
            } else {
                next_expr = generate_factor_ast(tokens);
            }
            expr = Expr::BinOp(next_op, Box::new(expr), Box::new(next_expr));
        } else {
            break;
        }
    }
    return expr;
}

fn generate_factor_ast(tokens: &mut TokenCursor) -> Expr {
    match tokens.peek() {
        Some(Token::OpenParen) => {
            tokens.next(); // consume opening parenthesis

            let expr = generate_expr_ast(tokens, BinOpPrecedenceLevel::lowest_level());

            if tokens.next() != Some(&Token::CloseParen) {
                err_display(
                    format!(
                        "expected closing parenthesis, found {:?}",
                        tokens.last().unwrap()
                    ),
                    tokens.get_last_ptr(),
                )
            }
            return expr;
        }
        Some(token) if token.to_un_op().is_some() => {
            let un_op = token.to_un_op().unwrap();
            tokens.next();
            let factor = generate_factor_ast(tokens);
            return Expr::UnOp(un_op, Box::new(factor));
        }
        Some(Token::IntLit { val }) => {
            let val_i32 = i32::from_str_radix(val, 10).unwrap();
            tokens.next();

            return Expr::Int(val_i32);
        }
        Some(Token::Identifier { val }) => {
            let val = val.clone();
            tokens.next();

            if tokens.peek() == Some(&Token::Op(Op::MinusMinus)) {
                tokens.next();
                return Expr::PostfixDec(val);
            } else if tokens.peek() == Some(&Token::Op(Op::PlusPlus)) {
                tokens.next();
                return Expr::PostfixInc(val);
            } else if tokens.peek() == Some(&Token::OpenParen) {
                tokens.next(); // consume the open paren
                let args = parse_function_args(tokens);
                if tokens.next() != Some(&Token::CloseParen) {
                    err_display(
                        format!(
                            "expected closing parenthesis, found {:?}",
                            tokens.last().unwrap()
                        ),
                        tokens.get_last_ptr(),
                    )
                }
                return Expr::FunctionCall(val, args);
            }
            return Expr::Var(val);
        }
        Some(Token::Op(op)) if *op == Op::PlusPlus || *op == Op::MinusMinus => {
            let op = op.clone();
            tokens.next();
            match tokens.next() {
                Some(Token::Identifier { val }) => {
                    if op == Op::PlusPlus {
                        return Expr::PrefixInc(val.clone());
                    } else {
                        return Expr::PrefixDec(val.clone());
                    }
                }
                _ => err_display(
                    "expected an identifier after the double inc/dec token",
                    tokens.get_last_ptr(),
                ),
            }
        }
        _ => err_display(
            format!("unexpected token: {:?}", tokens.peek()),
            tokens.get_last_ptr(),
        ),
    }
}

fn parse_function_args(tokens: &mut TokenCursor) -> Vec<Expr> {
    let mut args = Vec::new();

    if tokens.peek() == Some(&Token::CloseParen) {
        return Vec::new();
    }
    loop {
        args.push(generate_expr_ast(
            tokens,
            BinOpPrecedenceLevel::lowest_level(),
        ));
        if tokens.peek() == Some(&Token::Comma) {
            tokens.next(); // consume the comma
        } else {
            break;
        }
    }

    args
}

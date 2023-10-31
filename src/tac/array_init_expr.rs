use crate::{
    parser::expr_parser::{BinOp, Expr, ExprEnum},
    types::{VarSize, VarType},
};

use super::{
    expr::{generate_expr_tac, ValTarget},
    get_new_temp_name,
    tac_instr::TacInstr,
    CodeEnv, Identifier, TacVal,
};

pub fn gen_arr_init_expr_tac(
    arr_type: &VarType,
    arr_init_expr: &Expr,
    ptr_to_arr: Identifier,
    code_env: &CodeEnv,
) -> Vec<TacInstr> {
    let mut result = Vec::new();

    let exprs = match &arr_init_expr.content {
        ExprEnum::ArrInitExpr(x) => x,
        _ => unreachable!(),
    };

    let arr_type_size = arr_type.num_bytes() as i64;

    for expr in exprs {
        match &expr.content {
            ExprEnum::ArrInitExpr(_) => {
                let inner_type = match arr_type {
                    VarType::Arr(inner, _) => inner,
                    VarType::Fund(_) | VarType::Ptr(_) => unreachable!(),
                };

                let new_ptr = get_new_temp_name(VarSize::Quad);
                result.push(TacInstr::Copy(new_ptr, TacVal::Var(ptr_to_arr)));

                let instrs = gen_arr_init_expr_tac(inner_type, expr, new_ptr, code_env);
                result.extend(instrs);
            }
            _ => {
                let (expr_instrs, tac_val) = generate_expr_tac(expr, code_env, ValTarget::Generate);
                result.extend(expr_instrs);
                result.push(TacInstr::DerefStore(ptr_to_arr, tac_val));
            }
        }

        result.push(TacInstr::BinOp(
            ptr_to_arr,
            TacVal::Var(ptr_to_arr),
            TacVal::Lit(arr_type_size, VarSize::Quad),
            BinOp::Plus,
        ))
    }

    result
}
pub mod asm_gen;
pub mod binop;
pub mod functions;
pub mod putchar;
pub mod reg;
pub mod unop;
use std::collections::HashMap;

use crate::tac::{tac_instr::TacInstr, Identifier, TacVal};

use self::{
    binop::gen_binop_code, functions::generate_function_call_code, reg::Reg, unop::gen_unop_code,
};

pub struct RegisterAllocator {
    map: HashMap<Identifier, Location>,
}

impl RegisterAllocator {
    fn new(tac_instrs: &Vec<TacInstr>) -> (Self, usize) {
        let mut set_of_temporaries: Vec<Identifier> = Vec::new();

        for instr in tac_instrs {
            for ident in instr.get_read_identifiers() {
                if !set_of_temporaries.contains(&ident) {
                    panic!("read from temporary without first writing: {:?}", ident);
                }
            }
            if let Some(ident) = instr.get_written_identifier() {
                set_of_temporaries.push(ident);
            }
        }

        let mut map = HashMap::new();

        let mut bytes_needed = 0;

        for (index, t) in set_of_temporaries.iter().enumerate() {
            map.insert(*t, Location::Mem((index + 1) * 4));
            bytes_needed += 4;
        }

        (RegisterAllocator { map }, bytes_needed)
    }

    fn get_location(&self, temporary: Identifier) -> Location {
        return *self.map.get(&temporary).unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CCode {
    E,
    NE,
    L,
    LE,
    G,
    GE,
}

impl CCode {
    pub fn to_suffix(&self) -> String {
        match self {
            CCode::E => "e".to_owned(),
            CCode::NE => "ne".to_owned(),
            CCode::L => "l".to_owned(),
            CCode::LE => "le".to_owned(),
            CCode::G => "g".to_owned(),
            CCode::GE => "ge".to_owned(),
        }
    }
}

#[derive(Debug)]
pub enum X86Instr {
    Push { reg: Reg },
    Pop { reg: Reg },
    Mov { dst: Location, src: Location },
    MovImm { dst: Location, imm: i32 },
    Add { dst: Reg, src: Reg },
    Sub { dst: Reg, src: Reg },
    IMul { dst: Reg, src: Reg },
    SubImm { dst: Reg, imm: i32 },
    Cdq,               // convert double to quad, sign extends eax into edx:eax
    Idiv { src: Reg }, // divides rax by src, quotient stored in rax
    Label { name: String },
    Jmp { label: String },
    JmpCC { label: String, condition: CCode },
    SetCC { dst: Reg, condition: CCode },
    Test { src: Reg }, // does "test src, src", setting condition flags.
    Cmp { left: Reg, right: Reg },
    Not { dst: Reg }, // bitwise complement
    Neg { dst: Reg }, // negate the number (additive inverse)
    Call { name: String },
    Syscall,
}

#[derive(Clone, Copy, Debug)]
pub enum Location {
    Reg(Reg),
    Mem(usize), // usize represents offset from rbp
}

pub fn generate_x86_code(tac_instrs: &Vec<TacInstr>) -> Vec<X86Instr> {
    let mut result = Vec::new();

    let (reg_alloc, num_bytes_needed) = RegisterAllocator::new(tac_instrs);

    // FUNCTION PROLOGUE
    result.push(X86Instr::Push { reg: Reg::Rbp });
    result.push(X86Instr::Mov {
        dst: Location::Reg(Reg::Rbp),
        src: Location::Reg(Reg::Rsp),
    });
    result.push(X86Instr::SubImm {
        dst: Reg::Rsp,
        imm: num_bytes_needed as i32,
    });

    for instr in tac_instrs {
        gen_x86_for_tac(&mut result, instr, &reg_alloc);
    }

    // FUNCTION EPILOGUE
    result.push(X86Instr::Mov {
        dst: Location::Reg(Reg::Rsp),
        src: Location::Reg(Reg::Rbp),
    });
    result.push(X86Instr::Pop { reg: Reg::Rbp });

    result
}

fn gen_x86_for_tac(result: &mut Vec<X86Instr>, instr: &TacInstr, reg_alloc: &RegisterAllocator) {
    match instr {
        TacInstr::Exit(val) => {
            gen_load_val_code(result, val, Reg::Rdi, reg_alloc);
            // 60 is the syscall number for exit
            result.push(X86Instr::MovImm {
                dst: Location::Reg(Reg::Rax),
                imm: 60,
            });
            result.push(X86Instr::Syscall);
        }
        TacInstr::BinOp(dst_ident, val1, val2, op) => {
            gen_binop_code(result, dst_ident, val1, val2, *op, reg_alloc);
        }
        TacInstr::UnOp(dst_ident, val, op) => gen_unop_code(result, dst_ident, val, *op, reg_alloc),
        TacInstr::Copy(dst_ident, src_val) => {
            gen_load_val_code(result, src_val, Reg::Rdi, reg_alloc);
            result.push(X86Instr::Mov {
                dst: reg_alloc.get_location(*dst_ident),
                src: Location::Reg(Reg::Rdi),
            });
        }
        TacInstr::Label(label_name) => result.push(X86Instr::Label {
            name: label_name.clone(),
        }),
        TacInstr::Jmp(label_name) => result.push(X86Instr::Jmp {
            label: label_name.clone(),
        }),
        TacInstr::JmpZero(label_name, val) => {
            gen_load_val_code(result, val, Reg::Rdi, reg_alloc);
            result.push(X86Instr::Test { src: Reg::Rdi });
            result.push(X86Instr::JmpCC {
                label: label_name.clone(),
                condition: CCode::E,
            })
        }
        TacInstr::JmpNotZero(label_name, val) => {
            gen_load_val_code(result, val, Reg::Rdi, reg_alloc);
            result.push(X86Instr::Test { src: Reg::Rdi });
            result.push(X86Instr::JmpCC {
                label: label_name.clone(),
                condition: CCode::NE,
            })
        }
        TacInstr::Call(function_name, args, optional_ident) => {
            generate_function_call_code(result, function_name, args, *optional_ident, reg_alloc)
        }
    }
}

fn gen_load_val_code(
    result: &mut Vec<X86Instr>,
    val: &TacVal,
    reg: Reg,
    reg_alloc: &RegisterAllocator,
) {
    match val {
        TacVal::Lit(imm) => result.push(X86Instr::MovImm {
            dst: Location::Reg(reg),
            imm: *imm,
        }),
        TacVal::Var(var_ident) => {
            let loc = reg_alloc.get_location(*var_ident);
            result.push(X86Instr::Mov {
                dst: Location::Reg(reg),
                src: loc,
            });
        }
    }
}

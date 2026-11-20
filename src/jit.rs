use cranelift_codegen::ir::{
    condcodes::FloatCC, types, AbiParam, Block, InstBuilder, MemFlags, StackSlotData, StackSlotKind,
};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use std::collections::HashMap;

use crate::trace;

// cite: art dex_instruction_list.h#DEX_INSTRUCTION_LIST
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JitOp {
    Const {
        dst: u16,
        literal: i32,
    },
    Move {
        dst: u16,
        src: u16,
    },
    BinOp {
        dst: u16,
        a: u16,
        b: u16,
        kind: JitBinOp,
    },
    Goto {
        target: usize,
    },
    IfCmp {
        a: u16,
        b: u16,
        cmp: JitCmp,
        target: usize,
    },
    IfZero {
        a: u16,
        cmp: JitCmp,
        target: usize,
    },
    BinOpLit {
        dst: u16,
        src: u16,
        literal: i32,
        kind: JitLitOp,
    },
    UnOp {
        dst: u16,
        src: u16,
        kind: JitUnOp,
    },
    Return {
        src: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitLitOp {
    Add,
    RSub,
    Mul,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Ushr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitUnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitCmp {
    Eq,
    Ne,
    Lt,
    Ge,
    Gt,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitBinOp {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Ushr,
}

pub struct CompiledMethod {
    _module: JITModule,
    entry: *const u8,
}

impl CompiledMethod {
    pub fn call(&self, registers: &[i32]) -> i32 {
        let func: extern "C" fn(*const i32, usize) -> i32 =
            unsafe { std::mem::transmute(self.entry) };
        func(registers.as_ptr(), registers.len())
    }
}

// cite: art jit-inl.h#Jit::AddSamples
const HOT_METHOD_THRESHOLD: u32 = 8;

enum JitEntry {
    Counting(u32),
    Compiled(CompiledMethod),
    Unsupported,
}

#[derive(Default)]
pub struct JitCache {
    entries: std::collections::HashMap<(usize, u32), JitEntry>,
    object_entries: std::collections::HashMap<(usize, u32), ObjectJitEntry>,
}

impl JitCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance_integer(
        &mut self,
        dex_file_index: usize,
        code_offset: u32,
        instructions: &[u16],
        register_count: u16,
    ) -> bool {
        let key = (dex_file_index, code_offset);
        let entry = self.entries.entry(key).or_insert(JitEntry::Counting(0));

        match entry {
            JitEntry::Unsupported => false,
            JitEntry::Compiled(_) => true,
            JitEntry::Counting(count) => {
                *count += 1;
                if *count < HOT_METHOD_THRESHOLD {
                    return false;
                }
                match decode_integer_subset(instructions)
                    .and_then(|ops| compile(&ops, register_count))
                {
                    Some(compiled) => {
                        trace::event(
                            "jit",
                            format!(
                                "compiled method dex={dex_file_index} offset={code_offset} registers={register_count}"
                            ),
                        );
                        *entry = JitEntry::Compiled(compiled);
                        true
                    }
                    None => {
                        *entry = JitEntry::Unsupported;
                        false
                    }
                }
            }
        }
    }

    pub fn peek_integer(&self, dex_file_index: usize, code_offset: u32) -> Option<&CompiledMethod> {
        match self.entries.get(&(dex_file_index, code_offset)) {
            Some(JitEntry::Compiled(compiled)) => Some(compiled),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn run(
        &mut self,
        dex_file_index: usize,
        code_offset: u32,
        instructions: &[u16],
        register_count: u16,
        registers: &[i32],
    ) -> Option<i32> {
        if !self.advance_integer(dex_file_index, code_offset, instructions, register_count) {
            return None;
        }
        self.peek_integer(dex_file_index, code_offset)
            .map(|compiled| compiled.call(registers))
    }
}

// cite: art dex_instruction_list.h#DEX_INSTRUCTION_LIST
pub fn decode_integer_subset(instructions: &[u16]) -> Option<Vec<JitOp>> {
    let mut ops = Vec::new();
    let mut op_start_unit = Vec::new();
    let mut pending_targets: Vec<(usize, usize)> = Vec::new();
    let mut pc = 0usize;
    while pc < instructions.len() {
        let insn = instructions[pc];
        let opcode = (insn & 0x00ff) as u8;
        op_start_unit.push(pc);
        match opcode {
            0x12 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let literal = sign_extend_4(((insn >> 12) & 0x0f) as u8);
                ops.push(JitOp::Const { dst, literal });
                pc += 1;
            }
            0x13 => {
                let dst = u16::from((insn >> 8) as u8);
                let literal = i32::from(*instructions.get(pc + 1)? as i16);
                ops.push(JitOp::Const { dst, literal });
                pc += 2;
            }
            0x14 => {
                let dst = u16::from((insn >> 8) as u8);
                let low = u32::from(*instructions.get(pc + 1)?);
                let high = u32::from(*instructions.get(pc + 2)?);
                let literal = (low | (high << 16)) as i32;
                ops.push(JitOp::Const { dst, literal });
                pc += 3;
            }
            0x15 => {
                let dst = u16::from((insn >> 8) as u8);
                let literal = (i32::from(*instructions.get(pc + 1)? as i16)) << 16;
                ops.push(JitOp::Const { dst, literal });
                pc += 2;
            }
            0x01 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let src = u16::from((insn >> 12) & 0x0f);
                ops.push(JitOp::Move { dst, src });
                pc += 1;
            }
            0x90 | 0x91 | 0x92 | 0x95 | 0x96 | 0x97 | 0x98 | 0x99 | 0x9a => {
                let dst = u16::from((insn >> 8) as u8);
                let regs = *instructions.get(pc + 1)?;
                let a = regs & 0x00ff;
                let b = regs >> 8;
                let kind = int_binop_kind(opcode)?;
                ops.push(JitOp::BinOp { dst, a, b, kind });
                pc += 2;
            }
            0x28 => {
                let offset = (insn >> 8) as u8 as i8 as isize;
                let target = pc.checked_add_signed(offset)?;
                pending_targets.push((ops.len(), target));
                ops.push(JitOp::Goto { target: 0 });
                pc += 1;
            }
            0x32..=0x37 => {
                let a = u16::from((insn >> 8) & 0x0f);
                let b = u16::from((insn >> 12) & 0x0f);
                let offset = *instructions.get(pc + 1)? as i16 as isize;
                let target = pc.checked_add_signed(offset)?;
                let cmp = compare_branch_cmp(opcode)?;
                pending_targets.push((ops.len(), target));
                ops.push(JitOp::IfCmp {
                    a,
                    b,
                    cmp,
                    target: 0,
                });
                pc += 2;
            }
            0x38..=0x3d => {
                let a = u16::from((insn >> 8) as u8);
                let offset = *instructions.get(pc + 1)? as i16 as isize;
                let target = pc.checked_add_signed(offset)?;
                let cmp = zero_branch_cmp(opcode)?;
                pending_targets.push((ops.len(), target));
                ops.push(JitOp::IfZero { a, cmp, target: 0 });
                pc += 2;
            }
            0xb0 | 0xb1 | 0xb2 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let b = u16::from((insn >> 12) & 0x0f);
                let kind = match opcode {
                    0xb0 => JitBinOp::Add,
                    0xb1 => JitBinOp::Sub,
                    _ => JitBinOp::Mul,
                };
                ops.push(JitOp::BinOp {
                    dst,
                    a: dst,
                    b,
                    kind,
                });
                pc += 1;
            }
            0xd0 | 0xd1 | 0xd2 | 0xd5 | 0xd6 | 0xd7 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let src = u16::from((insn >> 12) & 0x0f);
                let literal = i32::from(*instructions.get(pc + 1)? as i16);
                let kind = int_lit_kind(opcode, 0xd0)?;
                ops.push(JitOp::BinOpLit {
                    dst,
                    src,
                    literal,
                    kind,
                });
                pc += 2;
            }
            // cite: art dex_instruction_list.h#ADD_INT_LIT8, SHL_INT_LIT8, SHR_INT_LIT8, USHR_INT_LIT8
            0xd8 | 0xd9 | 0xda | 0xdd | 0xde | 0xdf | 0xe0 | 0xe1 | 0xe2 => {
                let dst = u16::from((insn >> 8) as u8);
                let source_and_literal = *instructions.get(pc + 1)?;
                let src = source_and_literal & 0x00ff;
                let literal = i32::from((source_and_literal >> 8) as u8 as i8);
                let kind = int_lit_kind(opcode, 0xd8)?;
                ops.push(JitOp::BinOpLit {
                    dst,
                    src,
                    literal,
                    kind,
                });
                pc += 2;
            }
            0x7b | 0x7c => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let src = u16::from((insn >> 12) & 0x0f);
                let kind = if opcode == 0x7b {
                    JitUnOp::Neg
                } else {
                    JitUnOp::Not
                };
                ops.push(JitOp::UnOp { dst, src, kind });
                pc += 1;
            }
            0x0f => {
                let src = u16::from((insn >> 8) as u8);
                ops.push(JitOp::Return { src });
                pc += 1;
            }
            other => {
                trace::event("jit-reject", format!("opcode 0x{other:02x}"));
                return None;
            }
        }
    }

    let unit_to_op: HashMap<usize, usize> = op_start_unit
        .iter()
        .enumerate()
        .map(|(op_index, unit)| (*unit, op_index))
        .collect();
    for (op_index, target_unit) in pending_targets {
        let resolved = *unit_to_op.get(&target_unit)?;
        match &mut ops[op_index] {
            JitOp::Goto { target } | JitOp::IfCmp { target, .. } | JitOp::IfZero { target, .. } => {
                *target = resolved
            }
            _ => return None,
        }
    }

    Some(ops)
}

// cite: art dex_instruction_list.h#ADD_INT_LIT16, ADD_INT_LIT8, SHL_INT_LIT8, SHR_INT_LIT8, USHR_INT_LIT8
fn int_lit_kind(opcode: u8, base: u8) -> Option<JitLitOp> {
    Some(match opcode.checked_sub(base)? {
        0 => JitLitOp::Add,
        1 => JitLitOp::RSub,
        2 => JitLitOp::Mul,
        5 => JitLitOp::And,
        6 => JitLitOp::Or,
        7 => JitLitOp::Xor,
        8 if base == 0xd8 => JitLitOp::Shl,
        9 if base == 0xd8 => JitLitOp::Shr,
        10 if base == 0xd8 => JitLitOp::Ushr,
        _ => return None,
    })
}

fn int_binop_kind(opcode: u8) -> Option<JitBinOp> {
    Some(match opcode {
        0x90 => JitBinOp::Add,
        0x91 => JitBinOp::Sub,
        0x92 => JitBinOp::Mul,
        0x95 => JitBinOp::And,
        0x96 => JitBinOp::Or,
        0x97 => JitBinOp::Xor,
        0x98 => JitBinOp::Shl,
        0x99 => JitBinOp::Shr,
        0x9a => JitBinOp::Ushr,
        _ => return None,
    })
}

fn compare_branch_cmp(opcode: u8) -> Option<JitCmp> {
    Some(match opcode {
        0x32 => JitCmp::Eq,
        0x33 => JitCmp::Ne,
        0x34 => JitCmp::Lt,
        0x35 => JitCmp::Ge,
        0x36 => JitCmp::Gt,
        0x37 => JitCmp::Le,
        _ => return None,
    })
}

fn zero_branch_cmp(opcode: u8) -> Option<JitCmp> {
    Some(match opcode {
        0x38 => JitCmp::Eq,
        0x39 => JitCmp::Ne,
        0x3a => JitCmp::Lt,
        0x3b => JitCmp::Ge,
        0x3c => JitCmp::Gt,
        0x3d => JitCmp::Le,
        _ => return None,
    })
}

fn sign_extend_4(value: u8) -> i32 {
    let value = i32::from(value & 0x0f);
    if value & 0x08 != 0 {
        value - 16
    } else {
        value
    }
}

pub fn compile(ops: &[JitOp], register_count: u16) -> Option<CompiledMethod> {
    if ops.is_empty() || register_count == 0 {
        return None;
    }
    for op in ops {
        let ok = match op {
            JitOp::Const { dst, .. } => *dst < register_count,
            JitOp::Move { dst, src } => *dst < register_count && *src < register_count,
            JitOp::BinOp { dst, a, b, .. } => {
                *dst < register_count && *a < register_count && *b < register_count
            }
            JitOp::Return { src } => *src < register_count,
            JitOp::Goto { target } => *target < ops.len(),
            JitOp::IfCmp { a, b, target, .. } => {
                *a < register_count && *b < register_count && *target < ops.len()
            }
            JitOp::IfZero { a, target, .. } => *a < register_count && *target < ops.len(),
            JitOp::BinOpLit { dst, src, .. } => *dst < register_count && *src < register_count,
            JitOp::UnOp { dst, src, .. } => *dst < register_count && *src < register_count,
        };
        if !ok {
            return None;
        }
    }

    let mut is_leader = vec![false; ops.len()];
    is_leader[0] = true;
    for (index, op) in ops.iter().enumerate() {
        match op {
            JitOp::Goto { target } => {
                is_leader[*target] = true;
                if index + 1 < ops.len() {
                    is_leader[index + 1] = true;
                }
            }
            JitOp::IfCmp { target, .. } | JitOp::IfZero { target, .. } => {
                is_leader[*target] = true;
                if index + 1 < ops.len() {
                    is_leader[index + 1] = true;
                }
            }
            _ => {}
        }
    }

    let mut flag_builder = settings::builder();
    flag_builder.set("opt_level", "speed").ok()?;
    let isa_builder = cranelift_native::builder().ok()?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .ok()?;

    let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    let mut module = JITModule::new(builder);

    let mut ctx = module.make_context();
    let ptr = module.target_config().pointer_type();
    ctx.func.signature.params.push(AbiParam::new(ptr));
    ctx.func.signature.params.push(AbiParam::new(ptr));
    ctx.func.signature.returns.push(AbiParam::new(types::I32));

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut fb = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

    let vars: Vec<Variable> = (0..register_count)
        .map(|_| fb.declare_var(types::I32))
        .collect();

    let mut op_block: HashMap<usize, Block> = HashMap::new();
    for (index, leader) in is_leader.iter().enumerate() {
        if *leader {
            op_block.insert(index, fb.create_block());
        }
    }

    let entry = fb.create_block();
    fb.append_block_params_for_function_params(entry);
    fb.switch_to_block(entry);
    fb.seal_block(entry);
    let registers_ptr = fb.block_params(entry)[0];
    for (index, var) in vars.iter().enumerate() {
        let offset = i32::try_from(index * 4).ok()?;
        let value = fb.ins().load(
            types::I32,
            cranelift_codegen::ir::MemFlags::trusted(),
            registers_ptr,
            offset,
        );
        fb.def_var(*var, value);
    }
    fb.ins().jump(op_block[&0], &[]);

    let reg = |i: u16| vars[usize::from(i)];

    let mut current_terminated = true;
    for (index, op) in ops.iter().enumerate() {
        if let Some(block) = op_block.get(&index) {
            if !current_terminated {
                fb.ins().jump(*block, &[]);
            }
            fb.switch_to_block(*block);
            current_terminated = false;
        }

        match op {
            JitOp::Const { dst, literal } => {
                let value = fb.ins().iconst(types::I32, i64::from(*literal));
                fb.def_var(reg(*dst), value);
            }
            JitOp::Move { dst, src } => {
                let value = fb.use_var(reg(*src));
                fb.def_var(reg(*dst), value);
            }
            JitOp::BinOp { dst, a, b, kind } => {
                let lhs = fb.use_var(reg(*a));
                let rhs = fb.use_var(reg(*b));
                let value = match kind {
                    JitBinOp::Add => fb.ins().iadd(lhs, rhs),
                    JitBinOp::Sub => fb.ins().isub(lhs, rhs),
                    JitBinOp::Mul => fb.ins().imul(lhs, rhs),
                    JitBinOp::And => fb.ins().band(lhs, rhs),
                    JitBinOp::Or => fb.ins().bor(lhs, rhs),
                    JitBinOp::Xor => fb.ins().bxor(lhs, rhs),
                    JitBinOp::Shl | JitBinOp::Shr | JitBinOp::Ushr => {
                        let mask = fb.ins().iconst(types::I32, 0x1f);
                        let amount = fb.ins().band(rhs, mask);
                        match kind {
                            JitBinOp::Shl => fb.ins().ishl(lhs, amount),
                            JitBinOp::Shr => fb.ins().sshr(lhs, amount),
                            _ => fb.ins().ushr(lhs, amount),
                        }
                    }
                };
                fb.def_var(reg(*dst), value);
            }
            JitOp::Goto { target } => {
                fb.ins().jump(op_block[target], &[]);
                current_terminated = true;
            }
            JitOp::IfCmp { a, b, cmp, target } => {
                let lhs = fb.use_var(reg(*a));
                let rhs = fb.use_var(reg(*b));
                let cond = fb.ins().icmp(int_cc(*cmp), lhs, rhs);
                let fallthrough = op_block.get(&(index + 1))?;
                fb.ins()
                    .brif(cond, op_block[target], &[], *fallthrough, &[]);
                current_terminated = true;
            }
            JitOp::IfZero { a, cmp, target } => {
                let lhs = fb.use_var(reg(*a));
                let zero = fb.ins().iconst(types::I32, 0);
                let cond = fb.ins().icmp(int_cc(*cmp), lhs, zero);
                let fallthrough = op_block.get(&(index + 1))?;
                fb.ins()
                    .brif(cond, op_block[target], &[], *fallthrough, &[]);
                current_terminated = true;
            }
            JitOp::BinOpLit {
                dst,
                src,
                literal,
                kind,
            } => {
                let lhs = fb.use_var(reg(*src));
                let lit = fb.ins().iconst(types::I32, i64::from(*literal));
                let value = match kind {
                    JitLitOp::Add => fb.ins().iadd(lhs, lit),
                    JitLitOp::RSub => fb.ins().isub(lit, lhs),
                    JitLitOp::Mul => fb.ins().imul(lhs, lit),
                    JitLitOp::And => fb.ins().band(lhs, lit),
                    JitLitOp::Or => fb.ins().bor(lhs, lit),
                    JitLitOp::Xor => fb.ins().bxor(lhs, lit),
                    JitLitOp::Shl => {
                        let amount = fb.ins().iconst(types::I32, i64::from(*literal & 0x1f));
                        fb.ins().ishl(lhs, amount)
                    }
                    JitLitOp::Shr => {
                        let amount = fb.ins().iconst(types::I32, i64::from(*literal & 0x1f));
                        fb.ins().sshr(lhs, amount)
                    }
                    JitLitOp::Ushr => {
                        let amount = fb.ins().iconst(types::I32, i64::from(*literal & 0x1f));
                        fb.ins().ushr(lhs, amount)
                    }
                };
                fb.def_var(reg(*dst), value);
            }
            JitOp::UnOp { dst, src, kind } => {
                let operand = fb.use_var(reg(*src));
                let value = match kind {
                    JitUnOp::Neg => fb.ins().ineg(operand),
                    JitUnOp::Not => fb.ins().bnot(operand),
                };
                fb.def_var(reg(*dst), value);
            }
            JitOp::Return { src } => {
                let value = fb.use_var(reg(*src));
                fb.ins().return_(&[value]);
                current_terminated = true;
            }
        }
    }

    if !current_terminated {
        return None;
    }

    for block in op_block.values() {
        fb.seal_block(*block);
    }
    fb.finalize();

    let func_id = module
        .declare_function("jit_method", Linkage::Export, &ctx.func.signature)
        .ok()?;
    module.define_function(func_id, &mut ctx).ok()?;
    module.clear_context(&mut ctx);
    module.finalize_definitions().ok()?;

    let entry = module.get_finalized_function(func_id);
    Some(CompiledMethod {
        _module: module,
        entry,
    })
}

fn int_cc(cmp: JitCmp) -> cranelift_codegen::ir::condcodes::IntCC {
    use cranelift_codegen::ir::condcodes::IntCC;
    match cmp {
        JitCmp::Eq => IntCC::Equal,
        JitCmp::Ne => IntCC::NotEqual,
        JitCmp::Lt => IntCC::SignedLessThan,
        JitCmp::Ge => IntCC::SignedGreaterThanOrEqual,
        JitCmp::Gt => IntCC::SignedGreaterThan,
        JitCmp::Le => IntCC::SignedLessThanOrEqual,
    }
}

#[cfg(test)]
pub struct CompiledTrampolineCall {
    _module: JITModule,
    entry: *const u8,
}

#[cfg(test)]
impl CompiledTrampolineCall {
    pub unsafe fn call(&self, ctx: *mut std::ffi::c_void, arg: i32) -> i32 {
        let func: extern "C" fn(*mut std::ffi::c_void, i32) -> i32 =
            std::mem::transmute(self.entry);
        func(ctx, arg)
    }
}

#[cfg(test)]
pub fn compile_trampoline_call(
    trampoline: extern "C" fn(*mut std::ffi::c_void, i32) -> i32,
) -> Option<CompiledTrampolineCall> {
    let mut flag_builder = settings::builder();
    flag_builder.set("opt_level", "speed").ok()?;
    let isa = cranelift_native::builder()
        .ok()?
        .finish(settings::Flags::new(flag_builder))
        .ok()?;

    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    builder.symbol("rt_trampoline", trampoline as *const u8);
    let mut module = JITModule::new(builder);
    let ptr = module.target_config().pointer_type();

    let mut tramp_sig = module.make_signature();
    tramp_sig.params.push(AbiParam::new(ptr));
    tramp_sig.params.push(AbiParam::new(types::I32));
    tramp_sig.returns.push(AbiParam::new(types::I32));
    let tramp_id = module
        .declare_function("rt_trampoline", Linkage::Import, &tramp_sig)
        .ok()?;

    let mut ctx = module.make_context();
    ctx.func.signature.params.push(AbiParam::new(ptr));
    ctx.func.signature.params.push(AbiParam::new(types::I32));
    ctx.func.signature.returns.push(AbiParam::new(types::I32));

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut fb = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
    let tramp_ref = module.declare_func_in_func(tramp_id, fb.func);

    let entry = fb.create_block();
    fb.append_block_params_for_function_params(entry);
    fb.switch_to_block(entry);
    fb.seal_block(entry);
    let ctx_arg = fb.block_params(entry)[0];
    let arg = fb.block_params(entry)[1];
    let call = fb.ins().call(tramp_ref, &[ctx_arg, arg]);
    let result = fb.inst_results(call)[0];
    fb.ins().return_(&[result]);
    fb.finalize();

    let func_id = module
        .declare_function("tramp_entry", Linkage::Export, &ctx.func.signature)
        .ok()?;
    module.define_function(func_id, &mut ctx).ok()?;
    module.clear_context(&mut ctx);
    module.finalize_definitions().ok()?;

    let entry = module.get_finalized_function(func_id);
    Some(CompiledTrampolineCall {
        _module: module,
        entry,
    })
}

#[derive(Clone, Copy)]
pub struct ObjectMethodAbi {
    pub get_int: extern "C" fn(env: *mut std::ffi::c_void, reg: u32, pc: u32) -> i64,
    pub set_int: extern "C" fn(env: *mut std::ffi::c_void, reg: u32, val: i32),
    pub get_long:
        extern "C" fn(env: *mut std::ffi::c_void, reg: u32, pc: u32, status: *mut i32) -> i64,
    pub set_long: extern "C" fn(env: *mut std::ffi::c_void, reg: u32, val: i64),
    pub get_float_bits: extern "C" fn(env: *mut std::ffi::c_void, reg: u32, pc: u32) -> i64,
    pub set_float_bits: extern "C" fn(env: *mut std::ffi::c_void, reg: u32, bits: i32),
    pub iget_int: extern "C" fn(
        env: *mut std::ffi::c_void,
        dst: u32,
        obj: u32,
        field_index: u32,
        pc: u32,
    ) -> i32,
    pub aget:
        extern "C" fn(env: *mut std::ffi::c_void, dst: u32, array: u32, index: u32, pc: u32) -> i32,
    pub move_reg: extern "C" fn(env: *mut std::ffi::c_void, dst: u32, src: u32),
    pub iget_object: extern "C" fn(
        env: *mut std::ffi::c_void,
        dst: u32,
        obj: u32,
        field_index: u32,
        pc: u32,
    ) -> i32,
    pub iput_field: extern "C" fn(
        env: *mut std::ffi::c_void,
        val: u32,
        obj: u32,
        field_index: u32,
        pc: u32,
    ) -> i32,
    pub sget_int:
        extern "C" fn(env: *mut std::ffi::c_void, dst: u32, field_index: u32, pc: u32) -> i32,
    pub sget_object:
        extern "C" fn(env: *mut std::ffi::c_void, dst: u32, field_index: u32, pc: u32) -> i32,
    pub invoke: extern "C" fn(env: *mut std::ffi::c_void, pc: u32) -> i32,
    pub move_result: extern "C" fn(env: *mut std::ffi::c_void, dst: u32, wide: u32, pc: u32) -> i32,
    pub return_int: extern "C" fn(env: *mut std::ffi::c_void, src: u32, pc: u32) -> i32,
    pub return_void: extern "C" fn(env: *mut std::ffi::c_void),
    pub return_object: extern "C" fn(env: *mut std::ffi::c_void, src: u32),
    pub new_instance:
        extern "C" fn(env: *mut std::ffi::c_void, dst: u32, type_index: u32, pc: u32) -> i32,
    pub const_string:
        extern "C" fn(env: *mut std::ffi::c_void, dst: u32, string_index: u32, pc: u32) -> i32,
    pub const_class:
        extern "C" fn(env: *mut std::ffi::c_void, dst: u32, type_index: u32, pc: u32) -> i32,
    pub instance_of: extern "C" fn(
        env: *mut std::ffi::c_void,
        dst: u32,
        reg: u32,
        type_index: u32,
        pc: u32,
    ) -> i32,
    pub bail: extern "C" fn(env: *mut std::ffi::c_void, pc: u32) -> i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectOp {
    Const {
        dst: u16,
        literal: i32,
    },
    BinOp {
        dst: u16,
        a: u16,
        b: u16,
        kind: JitBinOp,
    },
    BinOpLit {
        dst: u16,
        src: u16,
        literal: i32,
        kind: JitLitOp,
        pc: u32,
    },
    UnOp {
        dst: u16,
        src: u16,
        kind: JitUnOp,
        pc: u32,
    },
    Move {
        dst: u16,
        src: u16,
    },
    IgetInt {
        dst: u16,
        obj: u16,
        field_index: u32,
        pc: u32,
    },
    ArrayGet {
        dst: u16,
        array: u16,
        index: u16,
        pc: u32,
    },
    IgetObject {
        dst: u16,
        obj: u16,
        field_index: u32,
        pc: u32,
    },
    IputField {
        val: u16,
        obj: u16,
        field_index: u32,
        pc: u32,
    },
    SgetInt {
        dst: u16,
        field_index: u32,
        pc: u32,
    },
    SgetObject {
        dst: u16,
        field_index: u32,
        pc: u32,
    },
    NewInstance {
        dst: u16,
        type_index: u32,
        pc: u32,
    },
    ConstString {
        dst: u16,
        string_index: u32,
        pc: u32,
    },
    ConstClass {
        dst: u16,
        type_index: u32,
        pc: u32,
    },
    InstanceOf {
        dst: u16,
        reg: u16,
        type_index: u32,
        pc: u32,
    },
    Goto {
        target: usize,
    },
    IfCmp {
        a: u16,
        b: u16,
        cmp: JitCmp,
        target: usize,
        pc: u32,
    },
    IfZero {
        a: u16,
        cmp: JitCmp,
        target: usize,
        pc: u32,
    },
    // cite: art dex_instruction_list.h#THROW
    Bail {
        pc: u32,
    },
    Invoke {
        pc: u32,
    },
    MoveResult {
        dst: u16,
        wide: bool,
        pc: u32,
    },
    Return {
        src: u16,
        pc: u32,
    },
    ReturnVoid,
    ReturnObject {
        src: u16,
    },
    LongConst {
        dst: u16,
        value: i64,
    },
    LongBinOp {
        dst: u16,
        a: u16,
        b: u16,
        kind: JitLongBinOp,
    },
    LongShift {
        dst: u16,
        src: u16,
        amount: u16,
        kind: JitShiftOp,
        pc: u32,
    },
    IntToLong {
        dst: u16,
        src: u16,
        pc: u32,
    },
    LongToInt {
        dst: u16,
        src: u16,
        pc: u32,
    },
    FloatBits {
        dst: u16,
        src: u16,
        pc: u32,
    },
    BitsToFloat {
        dst: u16,
        src: u16,
        pc: u32,
    },
    FloatBinOp {
        dst: u16,
        a: u16,
        b: u16,
        kind: JitFloatOp,
        pc: u32,
    },
    CmpFloat {
        dst: u16,
        a: u16,
        b: u16,
        bias: JitFloatCmpBias,
        pc: u32,
    },
    Convert {
        dst: u16,
        src: u16,
        kind: JitConvKind,
        pc: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitFloatOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitFloatCmpBias {
    Less,
    Greater,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitConvKind {
    IntToFloat,
    FloatToInt,
    IntToByte,
    IntToChar,
    IntToShort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitLongBinOp {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitShiftOp {
    Shl,
    Shr,
    Ushr,
}

pub struct CompiledObjectMethod {
    _module: JITModule,
    entry: *const u8,
}

impl CompiledObjectMethod {
    pub unsafe fn call(&self, env: *mut std::ffi::c_void) -> i32 {
        let func: extern "C" fn(*mut std::ffi::c_void) -> i32 = std::mem::transmute(self.entry);
        func(env)
    }
}
fn object_int_stable_registers(ops: &[ObjectOp], first_param_reg: u16) -> Vec<bool> {
    let mut max_reg: u16 = 0;
    {
        let mut note = |reg: u16| {
            if reg > max_reg {
                max_reg = reg;
            }
        };
        for op in ops {
            match op {
                ObjectOp::Const { dst, .. } => note(*dst),
                ObjectOp::BinOp { dst, a, b, .. } => {
                    note(*dst);
                    note(*a);
                    note(*b);
                }
                ObjectOp::BinOpLit { dst, src, .. } | ObjectOp::UnOp { dst, src, .. } => {
                    note(*dst);
                    note(*src);
                }
                ObjectOp::NewInstance { dst, .. }
                | ObjectOp::ConstString { dst, .. }
                | ObjectOp::ConstClass { dst, .. } => note(*dst),
                ObjectOp::InstanceOf { dst, reg, .. } => {
                    note(*dst);
                    note(*reg);
                }
                ObjectOp::Move { dst, src } => {
                    note(*dst);
                    note(*src);
                }
                ObjectOp::IgetInt { dst, obj, .. } | ObjectOp::IgetObject { dst, obj, .. } => {
                    note(*dst);
                    note(*obj);
                }
                ObjectOp::ArrayGet {
                    dst, array, index, ..
                } => {
                    note(*dst);
                    note(*array);
                    note(*index);
                }
                ObjectOp::IputField { val, obj, .. } => {
                    note(*val);
                    note(*obj);
                }
                ObjectOp::SgetInt { dst, .. } | ObjectOp::SgetObject { dst, .. } => note(*dst),
                ObjectOp::IfCmp { a, b, .. } => {
                    note(*a);
                    note(*b);
                }
                ObjectOp::IfZero { a, .. } => note(*a),
                ObjectOp::MoveResult { dst, wide, .. } => {
                    note(*dst);
                    if *wide {
                        note(dst.saturating_add(1));
                    }
                }
                ObjectOp::Return { src, .. } | ObjectOp::ReturnObject { src } => note(*src),
                ObjectOp::LongConst { dst, .. } => note(*dst),
                ObjectOp::LongBinOp { dst, a, b, .. } => {
                    note(*dst);
                    note(*a);
                    note(*b);
                }
                ObjectOp::FloatBinOp { dst, a, b, .. } => {
                    note(*dst);
                    note(*a);
                    note(*b);
                }
                ObjectOp::CmpFloat { dst, a, b, .. } => {
                    note(*dst);
                    note(*a);
                    note(*b);
                }
                ObjectOp::Convert { dst, src, .. } => {
                    note(*dst);
                    note(*src);
                }
                ObjectOp::LongShift {
                    dst, src, amount, ..
                } => {
                    note(*dst);
                    note(*src);
                    note(*amount);
                }
                ObjectOp::IntToLong { dst, src, .. } | ObjectOp::LongToInt { dst, src, .. } => {
                    note(*dst);
                    note(*src);
                }
                ObjectOp::FloatBits { dst, src, .. } | ObjectOp::BitsToFloat { dst, src, .. } => {
                    note(*dst);
                    note(*src);
                }
                ObjectOp::Invoke { .. }
                | ObjectOp::Goto { .. }
                | ObjectOp::Bail { .. }
                | ObjectOp::ReturnVoid => {}
            }
        }
    }

    let mut int_use = vec![false; max_reg as usize + 1];
    for op in ops {
        match op {
            ObjectOp::Const { dst, .. } => int_use[*dst as usize] = true,
            ObjectOp::BinOp { dst, a, b, .. } => {
                int_use[*dst as usize] = true;
                int_use[*a as usize] = true;
                int_use[*b as usize] = true;
            }
            ObjectOp::IgetInt { dst, .. } => int_use[*dst as usize] = true,
            ObjectOp::InstanceOf { dst, .. } => int_use[*dst as usize] = true,
            ObjectOp::SgetInt { dst, .. } => int_use[*dst as usize] = true,
            ObjectOp::IfCmp { a, b, .. } => {
                int_use[*a as usize] = true;
                int_use[*b as usize] = true;
            }
            ObjectOp::IfZero { a, .. } => int_use[*a as usize] = true,
            ObjectOp::Return { src, .. } => int_use[*src as usize] = true,
            ObjectOp::BinOpLit { dst, src, .. } | ObjectOp::UnOp { dst, src, .. } => {
                int_use[*dst as usize] = true;
                int_use[*src as usize] = true;
            }
            ObjectOp::CmpFloat { dst, .. } => int_use[*dst as usize] = true,
            _ => {}
        }
    }

    let mut disqualified = vec![false; max_reg as usize + 1];
    for op in ops {
        match op {
            ObjectOp::Const { .. }
            | ObjectOp::BinOp { .. }
            | ObjectOp::SgetInt { .. }
            | ObjectOp::IfCmp { .. }
            | ObjectOp::IfZero { .. }
            | ObjectOp::Return { .. }
            | ObjectOp::Invoke { .. }
            | ObjectOp::Goto { .. }
            | ObjectOp::Bail { .. }
            | ObjectOp::BinOpLit { .. }
            | ObjectOp::UnOp { .. }
            | ObjectOp::ReturnVoid => {}
            ObjectOp::IgetInt { obj, .. } => disqualified[*obj as usize] = true,
            ObjectOp::IgetObject { dst, obj, .. } => {
                disqualified[*dst as usize] = true;
                disqualified[*obj as usize] = true;
            }
            ObjectOp::ArrayGet {
                dst, array, index, ..
            } => {
                disqualified[*dst as usize] = true;
                disqualified[*array as usize] = true;
                disqualified[*index as usize] = true;
            }
            ObjectOp::IputField { val, obj, .. } => {
                disqualified[*val as usize] = true;
                disqualified[*obj as usize] = true;
            }
            ObjectOp::SgetObject { dst, .. }
            | ObjectOp::NewInstance { dst, .. }
            | ObjectOp::ConstString { dst, .. }
            | ObjectOp::ConstClass { dst, .. } => disqualified[*dst as usize] = true,
            ObjectOp::InstanceOf { reg, .. } => disqualified[*reg as usize] = true,
            ObjectOp::Move { dst, src } => {
                disqualified[*dst as usize] = true;
                disqualified[*src as usize] = true;
            }
            ObjectOp::MoveResult { dst, wide, .. } => {
                disqualified[*dst as usize] = true;
                if *wide {
                    disqualified[dst.saturating_add(1) as usize] = true;
                }
            }
            ObjectOp::ReturnObject { src } => disqualified[*src as usize] = true,
            ObjectOp::LongConst { dst, .. } => disqualified[*dst as usize] = true,
            ObjectOp::LongBinOp { dst, a, b, .. } => {
                disqualified[*dst as usize] = true;
                disqualified[*a as usize] = true;
                disqualified[*b as usize] = true;
            }
            ObjectOp::LongShift { dst, src, .. } => {
                disqualified[*dst as usize] = true;
                disqualified[*src as usize] = true;
            }
            ObjectOp::IntToLong { dst, .. } => disqualified[*dst as usize] = true,
            ObjectOp::LongToInt { src, .. } => disqualified[*src as usize] = true,
            ObjectOp::FloatBits { src, .. } => disqualified[*src as usize] = true,
            ObjectOp::BitsToFloat { dst, .. } => disqualified[*dst as usize] = true,
            ObjectOp::FloatBinOp { dst, a, b, .. } => {
                disqualified[*dst as usize] = true;
                disqualified[*a as usize] = true;
                disqualified[*b as usize] = true;
            }
            ObjectOp::CmpFloat { a, b, .. } => {
                disqualified[*a as usize] = true;
                disqualified[*b as usize] = true;
            }
            ObjectOp::Convert { dst, src, kind, .. } => match kind {
                JitConvKind::IntToFloat => disqualified[*dst as usize] = true,
                JitConvKind::FloatToInt => disqualified[*src as usize] = true,
                JitConvKind::IntToByte | JitConvKind::IntToChar | JitConvKind::IntToShort => {}
            },
        }
    }

    (0..=max_reg as usize)
        .map(|reg| int_use[reg] && !disqualified[reg] && (reg as u16) < first_param_reg)
        .collect()
}

fn object_long_stable_registers(
    ops: &[ObjectOp],
    int_stable: &[bool],
    first_param_reg: u16,
) -> Vec<bool> {
    let mut is_long = vec![false; int_stable.len()];
    let mut has_non_long_touch = vec![false; int_stable.len()];
    let note_long = |registers: &mut [bool], reg: u16| {
        if let Some(slot) = registers.get_mut(reg as usize) {
            *slot = true;
        }
    };
    let note_non_long = |registers: &mut [bool], reg: u16| {
        if let Some(slot) = registers.get_mut(reg as usize) {
            *slot = true;
        }
    };
    for op in ops {
        match op {
            ObjectOp::Const { dst, .. } => note_non_long(&mut has_non_long_touch, *dst),
            ObjectOp::BinOp { dst, a, b, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *a);
                note_non_long(&mut has_non_long_touch, *b);
            }
            ObjectOp::BinOpLit { dst, src, .. } | ObjectOp::UnOp { dst, src, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *src);
            }
            ObjectOp::NewInstance { dst, .. }
            | ObjectOp::ConstString { dst, .. }
            | ObjectOp::ConstClass { dst, .. }
            | ObjectOp::SgetInt { dst, .. }
            | ObjectOp::SgetObject { dst, .. } => note_non_long(&mut has_non_long_touch, *dst),
            ObjectOp::InstanceOf { dst, reg, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *reg);
            }
            ObjectOp::Move { dst, src } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *src);
            }
            ObjectOp::IgetInt { dst, obj, .. } | ObjectOp::IgetObject { dst, obj, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *obj);
            }
            ObjectOp::ArrayGet {
                dst, array, index, ..
            } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *array);
                note_non_long(&mut has_non_long_touch, *index);
            }
            ObjectOp::IputField { val, obj, .. } => {
                note_non_long(&mut has_non_long_touch, *val);
                note_non_long(&mut has_non_long_touch, *obj);
            }
            ObjectOp::IfCmp { a, b, .. } => {
                note_non_long(&mut has_non_long_touch, *a);
                note_non_long(&mut has_non_long_touch, *b);
            }
            ObjectOp::IfZero { a, .. } => note_non_long(&mut has_non_long_touch, *a),
            ObjectOp::MoveResult { dst, wide, .. } => {
                if *wide {
                    note_long(&mut is_long, *dst);
                } else {
                    note_non_long(&mut has_non_long_touch, *dst);
                }
            }
            ObjectOp::ReturnObject { src } => note_non_long(&mut has_non_long_touch, *src),
            ObjectOp::LongConst { dst, .. } => {
                note_long(&mut is_long, *dst);
            }
            ObjectOp::LongBinOp { dst, a, b, .. } => {
                note_long(&mut is_long, *dst);
                note_long(&mut is_long, *a);
                note_long(&mut is_long, *b);
            }
            ObjectOp::LongShift {
                dst, src, amount, ..
            } => {
                note_long(&mut is_long, *dst);
                note_long(&mut is_long, *src);
                note_non_long(&mut has_non_long_touch, *amount);
            }
            ObjectOp::IntToLong { dst, src, .. } => {
                note_long(&mut is_long, *dst);
                note_non_long(&mut has_non_long_touch, *src);
            }
            ObjectOp::LongToInt { dst, src, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_long(&mut is_long, *src);
            }
            ObjectOp::FloatBits { dst, src, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *src);
            }
            ObjectOp::BitsToFloat { dst, src, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *src);
            }
            ObjectOp::FloatBinOp { dst, a, b, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *a);
                note_non_long(&mut has_non_long_touch, *b);
            }
            ObjectOp::CmpFloat { dst, a, b, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *a);
                note_non_long(&mut has_non_long_touch, *b);
            }
            ObjectOp::Convert { dst, src, .. } => {
                note_non_long(&mut has_non_long_touch, *dst);
                note_non_long(&mut has_non_long_touch, *src);
            }
            ObjectOp::Return { .. }
            | ObjectOp::Invoke { .. }
            | ObjectOp::Goto { .. }
            | ObjectOp::Bail { .. }
            | ObjectOp::ReturnVoid => {}
        }
    }
    (0..int_stable.len())
        .map(|reg| {
            is_long[reg]
                && !has_non_long_touch[reg]
                && !int_stable[reg]
                && (reg as u16) < first_param_reg
        })
        .collect()
}

pub fn compile_object_method(
    ops: &[ObjectOp],
    first_param_reg: u16,
    abi: ObjectMethodAbi,
) -> Option<CompiledObjectMethod> {
    if ops.is_empty() {
        return None;
    }
    for op in ops {
        let target = match op {
            ObjectOp::Goto { target }
            | ObjectOp::IfCmp { target, .. }
            | ObjectOp::IfZero { target, .. } => Some(*target),
            _ => None,
        };
        if let Some(target) = target {
            if target >= ops.len() {
                return None;
            }
        }
    }

    let mut is_leader = vec![false; ops.len()];
    is_leader[0] = true;
    for (index, op) in ops.iter().enumerate() {
        let (branch_target, terminates) = match op {
            ObjectOp::Goto { target } => (Some(*target), true),
            ObjectOp::IfCmp { target, .. } | ObjectOp::IfZero { target, .. } => {
                (Some(*target), true)
            }
            ObjectOp::Return { .. } | ObjectOp::ReturnVoid | ObjectOp::ReturnObject { .. } => {
                (None, true)
            }
            ObjectOp::Bail { .. } => (None, true),
            _ => (None, false),
        };
        if let Some(target) = branch_target {
            is_leader[target] = true;
        }
        if terminates && index + 1 < ops.len() {
            is_leader[index + 1] = true;
        }
    }

    let mut flag_builder = settings::builder();
    flag_builder.set("opt_level", "speed").ok()?;
    let isa = cranelift_native::builder()
        .ok()?
        .finish(settings::Flags::new(flag_builder))
        .ok()?;

    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    builder.symbol("rt_get_int", abi.get_int as *const u8);
    builder.symbol("rt_set_int", abi.set_int as *const u8);
    builder.symbol("rt_get_long", abi.get_long as *const u8);
    builder.symbol("rt_set_long", abi.set_long as *const u8);
    builder.symbol("rt_get_float_bits", abi.get_float_bits as *const u8);
    builder.symbol("rt_set_float_bits", abi.set_float_bits as *const u8);
    builder.symbol("rt_iget_int", abi.iget_int as *const u8);
    builder.symbol("rt_aget", abi.aget as *const u8);
    builder.symbol("rt_move", abi.move_reg as *const u8);
    builder.symbol("rt_iget_object", abi.iget_object as *const u8);
    builder.symbol("rt_iput_field", abi.iput_field as *const u8);
    builder.symbol("rt_sget_int", abi.sget_int as *const u8);
    builder.symbol("rt_sget_object", abi.sget_object as *const u8);
    builder.symbol("rt_invoke", abi.invoke as *const u8);
    builder.symbol("rt_move_result", abi.move_result as *const u8);
    builder.symbol("rt_return_int", abi.return_int as *const u8);
    builder.symbol("rt_return_void", abi.return_void as *const u8);
    builder.symbol("rt_return_object", abi.return_object as *const u8);
    builder.symbol("rt_new_instance", abi.new_instance as *const u8);
    builder.symbol("rt_const_string", abi.const_string as *const u8);
    builder.symbol("rt_const_class", abi.const_class as *const u8);
    builder.symbol("rt_instance_of", abi.instance_of as *const u8);
    builder.symbol("rt_bail", abi.bail as *const u8);
    let mut module = JITModule::new(builder);
    let ptr = module.target_config().pointer_type();

    let mut sig_p2_rl = module.make_signature();
    sig_p2_rl.params.push(AbiParam::new(ptr));
    sig_p2_rl.params.push(AbiParam::new(types::I32));
    sig_p2_rl.params.push(AbiParam::new(types::I32));
    sig_p2_rl.returns.push(AbiParam::new(types::I64));
    let mut sig_p2 = module.make_signature();
    sig_p2.params.push(AbiParam::new(ptr));
    sig_p2.params.push(AbiParam::new(types::I32));
    sig_p2.params.push(AbiParam::new(types::I32));
    let mut sig_p1 = module.make_signature();
    sig_p1.params.push(AbiParam::new(ptr));
    sig_p1.params.push(AbiParam::new(types::I32));
    let mut sig_p0 = module.make_signature();
    sig_p0.params.push(AbiParam::new(ptr));
    let mut sig_p1_r = module.make_signature();
    sig_p1_r.params.push(AbiParam::new(ptr));
    sig_p1_r.params.push(AbiParam::new(types::I32));
    sig_p1_r.returns.push(AbiParam::new(types::I32));
    let mut sig_p2_r = module.make_signature();
    sig_p2_r.params.push(AbiParam::new(ptr));
    sig_p2_r.params.push(AbiParam::new(types::I32));
    sig_p2_r.params.push(AbiParam::new(types::I32));
    sig_p2_r.returns.push(AbiParam::new(types::I32));
    let mut sig_p3_r = module.make_signature();
    sig_p3_r.params.push(AbiParam::new(ptr));
    sig_p3_r.params.push(AbiParam::new(types::I32));
    sig_p3_r.params.push(AbiParam::new(types::I32));
    sig_p3_r.params.push(AbiParam::new(types::I32));
    sig_p3_r.returns.push(AbiParam::new(types::I32));
    let mut sig_p4_r = module.make_signature();
    sig_p4_r.params.push(AbiParam::new(ptr));
    sig_p4_r.params.push(AbiParam::new(types::I32));
    sig_p4_r.params.push(AbiParam::new(types::I32));
    sig_p4_r.params.push(AbiParam::new(types::I32));
    sig_p4_r.params.push(AbiParam::new(types::I32));
    sig_p4_r.returns.push(AbiParam::new(types::I32));
    let mut sig_get_long = module.make_signature();
    sig_get_long.params.push(AbiParam::new(ptr));
    sig_get_long.params.push(AbiParam::new(types::I32));
    sig_get_long.params.push(AbiParam::new(types::I32));
    sig_get_long.params.push(AbiParam::new(ptr));
    sig_get_long.returns.push(AbiParam::new(types::I64));
    let mut sig_set_long = module.make_signature();
    sig_set_long.params.push(AbiParam::new(ptr));
    sig_set_long.params.push(AbiParam::new(types::I32));
    sig_set_long.params.push(AbiParam::new(types::I64));

    let get_id = module
        .declare_function("rt_get_int", Linkage::Import, &sig_p2_rl)
        .ok()?;
    let set_id = module
        .declare_function("rt_set_int", Linkage::Import, &sig_p2)
        .ok()?;
    let get_long_id = module
        .declare_function("rt_get_long", Linkage::Import, &sig_get_long)
        .ok()?;
    let set_long_id = module
        .declare_function("rt_set_long", Linkage::Import, &sig_set_long)
        .ok()?;
    let get_float_bits_id = module
        .declare_function("rt_get_float_bits", Linkage::Import, &sig_p2_rl)
        .ok()?;
    let set_float_bits_id = module
        .declare_function("rt_set_float_bits", Linkage::Import, &sig_p2)
        .ok()?;
    let iget_id = module
        .declare_function("rt_iget_int", Linkage::Import, &sig_p4_r)
        .ok()?;
    let aget_id = module
        .declare_function("rt_aget", Linkage::Import, &sig_p4_r)
        .ok()?;
    let move_id = module
        .declare_function("rt_move", Linkage::Import, &sig_p2)
        .ok()?;
    let iget_obj_id = module
        .declare_function("rt_iget_object", Linkage::Import, &sig_p4_r)
        .ok()?;
    let iput_id = module
        .declare_function("rt_iput_field", Linkage::Import, &sig_p4_r)
        .ok()?;
    let sget_int_id = module
        .declare_function("rt_sget_int", Linkage::Import, &sig_p3_r)
        .ok()?;
    let sget_obj_id = module
        .declare_function("rt_sget_object", Linkage::Import, &sig_p3_r)
        .ok()?;
    let invoke_id = module
        .declare_function("rt_invoke", Linkage::Import, &sig_p1_r)
        .ok()?;
    let move_result_id = module
        .declare_function("rt_move_result", Linkage::Import, &sig_p3_r)
        .ok()?;
    let return_int_id = module
        .declare_function("rt_return_int", Linkage::Import, &sig_p2_r)
        .ok()?;
    let return_void_id = module
        .declare_function("rt_return_void", Linkage::Import, &sig_p0)
        .ok()?;
    let return_object_id = module
        .declare_function("rt_return_object", Linkage::Import, &sig_p1)
        .ok()?;
    let new_instance_id = module
        .declare_function("rt_new_instance", Linkage::Import, &sig_p3_r)
        .ok()?;
    let const_string_id = module
        .declare_function("rt_const_string", Linkage::Import, &sig_p3_r)
        .ok()?;
    let const_class_id = module
        .declare_function("rt_const_class", Linkage::Import, &sig_p3_r)
        .ok()?;
    let instance_of_id = module
        .declare_function("rt_instance_of", Linkage::Import, &sig_p4_r)
        .ok()?;
    let bail_id = module
        .declare_function("rt_bail", Linkage::Import, &sig_p1_r)
        .ok()?;

    let mut ctx = module.make_context();
    ctx.func.signature.params.push(AbiParam::new(ptr));
    ctx.func.signature.returns.push(AbiParam::new(types::I32));

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut fb = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
    let get_ref = module.declare_func_in_func(get_id, fb.func);
    let set_ref = module.declare_func_in_func(set_id, fb.func);
    let get_long_ref = module.declare_func_in_func(get_long_id, fb.func);
    let set_long_ref = module.declare_func_in_func(set_long_id, fb.func);
    let get_float_bits_ref = module.declare_func_in_func(get_float_bits_id, fb.func);
    let set_float_bits_ref = module.declare_func_in_func(set_float_bits_id, fb.func);
    let iget_ref = module.declare_func_in_func(iget_id, fb.func);
    let aget_ref = module.declare_func_in_func(aget_id, fb.func);
    let move_ref = module.declare_func_in_func(move_id, fb.func);
    let iget_obj_ref = module.declare_func_in_func(iget_obj_id, fb.func);
    let iput_ref = module.declare_func_in_func(iput_id, fb.func);
    let sget_int_ref = module.declare_func_in_func(sget_int_id, fb.func);
    let sget_obj_ref = module.declare_func_in_func(sget_obj_id, fb.func);
    let invoke_ref = module.declare_func_in_func(invoke_id, fb.func);
    let move_result_ref = module.declare_func_in_func(move_result_id, fb.func);
    let return_int_ref = module.declare_func_in_func(return_int_id, fb.func);
    let return_void_ref = module.declare_func_in_func(return_void_id, fb.func);
    let return_object_ref = module.declare_func_in_func(return_object_id, fb.func);
    let new_instance_ref = module.declare_func_in_func(new_instance_id, fb.func);
    let const_string_ref = module.declare_func_in_func(const_string_id, fb.func);
    let const_class_ref = module.declare_func_in_func(const_class_id, fb.func);
    let instance_of_ref = module.declare_func_in_func(instance_of_id, fb.func);
    let bail_ref = module.declare_func_in_func(bail_id, fb.func);

    let entry = fb.create_block();
    fb.append_block_params_for_function_params(entry);
    let env = fb.block_params(entry)[0];

    let exit_block = fb.create_block();
    let exit_status = fb.append_block_param(exit_block, types::I32);

    let mut op_block: HashMap<usize, Block> = HashMap::new();
    for (index, leader) in is_leader.iter().enumerate() {
        if *leader {
            op_block.insert(index, fb.create_block());
        }
    }

    fb.switch_to_block(entry);
    fb.seal_block(entry);

    let stable = object_int_stable_registers(ops, first_param_reg);
    let mut shadow: Vec<Option<Variable>> = vec![None; stable.len()];
    for (register, is_stable) in stable.iter().enumerate() {
        if *is_stable {
            let var = fb.declare_var(types::I32);
            let zero = fb.ins().iconst(types::I32, 0);
            fb.def_var(var, zero);
            shadow[register] = Some(var);
        }
    }
    let long_stable = object_long_stable_registers(ops, &stable, first_param_reg);
    let mut long_shadow: Vec<Option<Variable>> = vec![None; long_stable.len()];
    for (register, is_stable) in long_stable.iter().enumerate() {
        if *is_stable {
            let var = fb.declare_var(types::I64);
            let zero = fb.ins().iconst(types::I64, 0);
            fb.def_var(var, zero);
            long_shadow[register] = Some(var);
        }
    }
    fb.ins().jump(op_block[&0], &[]);

    let reg = |fb: &mut FunctionBuilder, r: u16| fb.ins().iconst(types::I32, i64::from(r));
    let check_status =
        |fb: &mut FunctionBuilder, status: cranelift_codegen::ir::Value, exit: Block| {
            let cont = fb.create_block();
            fb.ins().brif(status, exit, &[status.into()], cont, &[]);
            fb.switch_to_block(cont);
            fb.seal_block(cont);
        };
    let read_reg = |fb: &mut FunctionBuilder, r: u16, pc: u32| -> cranelift_codegen::ir::Value {
        if let Some(var) = shadow[r as usize] {
            return fb.use_var(var);
        }
        let rr = fb.ins().iconst(types::I32, i64::from(r));
        let pcv = fb.ins().iconst(types::I32, i64::from(pc));
        let call = fb.ins().call(get_ref, &[env, rr, pcv]);
        let packed = fb.inst_results(call)[0];
        let status_wide = fb.ins().ushr_imm(packed, 32);
        let status = fb.ins().ireduce(types::I32, status_wide);
        let cont = fb.create_block();
        fb.ins()
            .brif(status, exit_block, &[status.into()], cont, &[]);
        fb.switch_to_block(cont);
        fb.seal_block(cont);
        fb.ins().ireduce(types::I32, packed)
    };
    let write_reg = |fb: &mut FunctionBuilder, r: u16, value: cranelift_codegen::ir::Value| {
        if let Some(var) = shadow[r as usize] {
            fb.def_var(var, value);
        } else {
            let rr = fb.ins().iconst(types::I32, i64::from(r));
            fb.ins().call(set_ref, &[env, rr, value]);
        }
    };
    let status_slot =
        fb.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 4, 2));
    let read_wide = |fb: &mut FunctionBuilder, r: u16, pc: u32| -> cranelift_codegen::ir::Value {
        if let Some(var) = long_shadow[r as usize] {
            return fb.use_var(var);
        }
        let rr = fb.ins().iconst(types::I32, i64::from(r));
        let pcv = fb.ins().iconst(types::I32, i64::from(pc));
        let addr = fb.ins().stack_addr(ptr, status_slot, 0);
        let call = fb.ins().call(get_long_ref, &[env, rr, pcv, addr]);
        let value = fb.inst_results(call)[0];
        let status = fb.ins().stack_load(types::I32, status_slot, 0);
        let cont = fb.create_block();
        fb.ins()
            .brif(status, exit_block, &[status.into()], cont, &[]);
        fb.switch_to_block(cont);
        fb.seal_block(cont);
        value
    };
    let write_wide = |fb: &mut FunctionBuilder, r: u16, value: cranelift_codegen::ir::Value| {
        if let Some(var) = long_shadow[r as usize] {
            fb.def_var(var, value);
        } else {
            let rr = fb.ins().iconst(types::I32, i64::from(r));
            fb.ins().call(set_long_ref, &[env, rr, value]);
        }
    };
    let flush_one = |fb: &mut FunctionBuilder, r: u16| {
        if let Some(var) = shadow[r as usize] {
            let value = fb.use_var(var);
            let rr = fb.ins().iconst(types::I32, i64::from(r));
            fb.ins().call(set_ref, &[env, rr, value]);
        }
        if let Some(var) = long_shadow[r as usize] {
            let value = fb.use_var(var);
            let rr = fb.ins().iconst(types::I32, i64::from(r));
            fb.ins().call(set_long_ref, &[env, rr, value]);
        }
    };
    let flush_all = |fb: &mut FunctionBuilder| {
        for (r, var) in shadow.iter().enumerate() {
            if let Some(var) = var {
                let value = fb.use_var(*var);
                let rr = fb.ins().iconst(types::I32, r as i64);
                fb.ins().call(set_ref, &[env, rr, value]);
            }
        }
        for (r, var) in long_shadow.iter().enumerate() {
            if let Some(var) = var {
                let value = fb.use_var(*var);
                let rr = fb.ins().iconst(types::I32, r as i64);
                fb.ins().call(set_long_ref, &[env, rr, value]);
            }
        }
    };
    let reload = |fb: &mut FunctionBuilder, r: u16| {
        if let Some(var) = shadow[r as usize] {
            let rr = fb.ins().iconst(types::I32, i64::from(r));
            let pcv = fb.ins().iconst(types::I32, 0);
            let call = fb.ins().call(get_ref, &[env, rr, pcv]);
            let packed = fb.inst_results(call)[0];
            let value = fb.ins().ireduce(types::I32, packed);
            fb.def_var(var, value);
        }
    };

    let op_pc = |op: &ObjectOp| -> u32 {
        match op {
            ObjectOp::IgetInt { pc, .. }
            | ObjectOp::IgetObject { pc, .. }
            | ObjectOp::IputField { pc, .. }
            | ObjectOp::SgetInt { pc, .. }
            | ObjectOp::SgetObject { pc, .. }
            | ObjectOp::Invoke { pc }
            | ObjectOp::MoveResult { pc, .. }
            | ObjectOp::Return { pc, .. }
            | ObjectOp::IfCmp { pc, .. }
            | ObjectOp::IfZero { pc, .. }
            | ObjectOp::BinOpLit { pc, .. }
            | ObjectOp::UnOp { pc, .. }
            | ObjectOp::Bail { pc, .. }
            | ObjectOp::NewInstance { pc, .. }
            | ObjectOp::ConstString { pc, .. }
            | ObjectOp::ConstClass { pc, .. }
            | ObjectOp::InstanceOf { pc, .. } => *pc,
            ObjectOp::CmpFloat { pc, .. } => *pc,
            _ => 0,
        }
    };

    let mut current_terminated = true;
    for (index, op) in ops.iter().enumerate() {
        if let Some(block) = op_block.get(&index) {
            if !current_terminated {
                fb.ins().jump(*block, &[]);
            }
            fb.switch_to_block(*block);
            current_terminated = false;
        }
        let pc = op_pc(op);
        match op {
            ObjectOp::Const { dst, literal } => {
                let v = fb.ins().iconst(types::I32, i64::from(*literal));
                write_reg(&mut fb, *dst, v);
            }
            ObjectOp::BinOp { dst, a, b, kind } => {
                let lhs = read_reg(&mut fb, *a, pc);
                let rhs = read_reg(&mut fb, *b, pc);
                let value = match kind {
                    JitBinOp::Add => fb.ins().iadd(lhs, rhs),
                    JitBinOp::Sub => fb.ins().isub(lhs, rhs),
                    JitBinOp::Mul => fb.ins().imul(lhs, rhs),
                    JitBinOp::And => fb.ins().band(lhs, rhs),
                    JitBinOp::Or => fb.ins().bor(lhs, rhs),
                    JitBinOp::Xor => fb.ins().bxor(lhs, rhs),
                    JitBinOp::Shl | JitBinOp::Shr | JitBinOp::Ushr => {
                        let mask = fb.ins().iconst(types::I32, 0x1f);
                        let amount = fb.ins().band(rhs, mask);
                        match kind {
                            JitBinOp::Shl => fb.ins().ishl(lhs, amount),
                            JitBinOp::Shr => fb.ins().sshr(lhs, amount),
                            _ => fb.ins().ushr(lhs, amount),
                        }
                    }
                };
                write_reg(&mut fb, *dst, value);
            }
            ObjectOp::LongConst { dst, value } => {
                let v = fb.ins().iconst(types::I64, *value);
                write_wide(&mut fb, *dst, v);
            }
            ObjectOp::LongBinOp { dst, a, b, kind } => {
                let lhs = read_wide(&mut fb, *a, pc);
                let rhs = read_wide(&mut fb, *b, pc);
                let value = match kind {
                    JitLongBinOp::Add => fb.ins().iadd(lhs, rhs),
                    JitLongBinOp::Sub => fb.ins().isub(lhs, rhs),
                    JitLongBinOp::Mul => fb.ins().imul(lhs, rhs),
                    JitLongBinOp::And => fb.ins().band(lhs, rhs),
                    JitLongBinOp::Or => fb.ins().bor(lhs, rhs),
                    JitLongBinOp::Xor => fb.ins().bxor(lhs, rhs),
                };
                write_wide(&mut fb, *dst, value);
            }
            ObjectOp::LongShift {
                dst,
                src,
                amount,
                kind,
                ..
            } => {
                let val = read_wide(&mut fb, *src, pc);
                let amt_i32 = read_reg(&mut fb, *amount, pc);
                let amt = fb.ins().uextend(types::I64, amt_i32);
                let mask = fb.ins().iconst(types::I64, 0x3f);
                let amt = fb.ins().band(amt, mask);
                let value = match kind {
                    JitShiftOp::Shl => fb.ins().ishl(val, amt),
                    JitShiftOp::Shr => fb.ins().sshr(val, amt),
                    JitShiftOp::Ushr => fb.ins().ushr(val, amt),
                };
                write_wide(&mut fb, *dst, value);
            }
            ObjectOp::IntToLong { dst, src, .. } => {
                let v_i32 = read_reg(&mut fb, *src, pc);
                let v = fb.ins().sextend(types::I64, v_i32);
                write_wide(&mut fb, *dst, v);
            }
            ObjectOp::LongToInt { dst, src, .. } => {
                let v = read_wide(&mut fb, *src, pc);
                let v_i32 = fb.ins().ireduce(types::I32, v);
                write_reg(&mut fb, *dst, v_i32);
            }
            ObjectOp::FloatBits { dst, src, .. } => {
                let rr = fb.ins().iconst(types::I32, i64::from(*src));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(get_float_bits_ref, &[env, rr, pcv]);
                let packed = fb.inst_results(call)[0];
                let status_wide = fb.ins().ushr_imm(packed, 32);
                let status = fb.ins().ireduce(types::I32, status_wide);
                let cont = fb.create_block();
                fb.ins()
                    .brif(status, exit_block, &[status.into()], cont, &[]);
                fb.switch_to_block(cont);
                fb.seal_block(cont);
                let bits = fb.ins().ireduce(types::I32, packed);
                write_reg(&mut fb, *dst, bits);
            }
            ObjectOp::BitsToFloat { dst, src, .. } => {
                let bits = read_reg(&mut fb, *src, pc);
                let rr = fb.ins().iconst(types::I32, i64::from(*dst));
                fb.ins().call(set_float_bits_ref, &[env, rr, bits]);
            }
            ObjectOp::FloatBinOp {
                dst, a, b, kind, ..
            } => {
                let read_f32_bits =
                    |fb: &mut FunctionBuilder, reg: u16| -> cranelift_codegen::ir::Value {
                        let rr = fb.ins().iconst(types::I32, i64::from(reg));
                        let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                        let call = fb.ins().call(get_float_bits_ref, &[env, rr, pcv]);
                        let packed = fb.inst_results(call)[0];
                        let status_wide = fb.ins().ushr_imm(packed, 32);
                        let status = fb.ins().ireduce(types::I32, status_wide);
                        let cont = fb.create_block();
                        fb.ins()
                            .brif(status, exit_block, &[status.into()], cont, &[]);
                        fb.switch_to_block(cont);
                        fb.seal_block(cont);
                        fb.ins().ireduce(types::I32, packed)
                    };
                let a_bits = read_f32_bits(&mut fb, *a);
                let b_bits = read_f32_bits(&mut fb, *b);
                let af = fb.ins().bitcast(types::F32, MemFlags::new(), a_bits);
                let bf = fb.ins().bitcast(types::F32, MemFlags::new(), b_bits);
                let rf = match kind {
                    JitFloatOp::Add => fb.ins().fadd(af, bf),
                    JitFloatOp::Sub => fb.ins().fsub(af, bf),
                    JitFloatOp::Mul => fb.ins().fmul(af, bf),
                    JitFloatOp::Div => fb.ins().fdiv(af, bf),
                };
                let r_bits = fb.ins().bitcast(types::I32, MemFlags::new(), rf);
                let rr = fb.ins().iconst(types::I32, i64::from(*dst));
                fb.ins().call(set_float_bits_ref, &[env, rr, r_bits]);
            }
            ObjectOp::CmpFloat {
                dst, a, b, bias, ..
            } => {
                let read_f32_bits =
                    |fb: &mut FunctionBuilder, reg: u16| -> cranelift_codegen::ir::Value {
                        let rr = fb.ins().iconst(types::I32, i64::from(reg));
                        let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                        let call = fb.ins().call(get_float_bits_ref, &[env, rr, pcv]);
                        let packed = fb.inst_results(call)[0];
                        let status_wide = fb.ins().ushr_imm(packed, 32);
                        let status = fb.ins().ireduce(types::I32, status_wide);
                        let cont = fb.create_block();
                        fb.ins()
                            .brif(status, exit_block, &[status.into()], cont, &[]);
                        fb.switch_to_block(cont);
                        fb.seal_block(cont);
                        fb.ins().ireduce(types::I32, packed)
                    };
                let a_bits = read_f32_bits(&mut fb, *a);
                let b_bits = read_f32_bits(&mut fb, *b);
                let af = fb.ins().bitcast(types::F32, MemFlags::new(), a_bits);
                let bf = fb.ins().bitcast(types::F32, MemFlags::new(), b_bits);
                let unordered = fb.ins().fcmp(FloatCC::Unordered, af, bf);
                let greater = fb.ins().fcmp(FloatCC::GreaterThan, af, bf);
                let equal = fb.ins().fcmp(FloatCC::Equal, af, bf);
                let one = fb.ins().iconst(types::I32, 1);
                let zero = fb.ins().iconst(types::I32, 0);
                let minus_one = fb.ins().iconst(types::I32, -1);
                let ordered = fb.ins().select(greater, one, minus_one);
                let ordered = fb.ins().select(equal, zero, ordered);
                let nan_value = match bias {
                    JitFloatCmpBias::Less => minus_one,
                    JitFloatCmpBias::Greater => one,
                };
                let value = fb.ins().select(unordered, nan_value, ordered);
                write_reg(&mut fb, *dst, value);
            }
            ObjectOp::Convert { dst, src, kind, .. } => match kind {
                JitConvKind::IntToFloat => {
                    let v = read_reg(&mut fb, *src, pc);
                    let f = fb.ins().fcvt_from_sint(types::F32, v);
                    let bits = fb.ins().bitcast(types::I32, MemFlags::new(), f);
                    let rr = fb.ins().iconst(types::I32, i64::from(*dst));
                    fb.ins().call(set_float_bits_ref, &[env, rr, bits]);
                }
                JitConvKind::FloatToInt => {
                    let rr = fb.ins().iconst(types::I32, i64::from(*src));
                    let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                    let call = fb.ins().call(get_float_bits_ref, &[env, rr, pcv]);
                    let packed = fb.inst_results(call)[0];
                    let status_wide = fb.ins().ushr_imm(packed, 32);
                    let status = fb.ins().ireduce(types::I32, status_wide);
                    let cont = fb.create_block();
                    fb.ins()
                        .brif(status, exit_block, &[status.into()], cont, &[]);
                    fb.switch_to_block(cont);
                    fb.seal_block(cont);
                    let bits = fb.ins().ireduce(types::I32, packed);
                    let f = fb.ins().bitcast(types::F32, MemFlags::new(), bits);
                    let i = fb.ins().fcvt_to_sint_sat(types::I32, f);
                    write_reg(&mut fb, *dst, i);
                }
                JitConvKind::IntToByte => {
                    let v = read_reg(&mut fb, *src, pc);
                    let r = fb.ins().ireduce(types::I8, v);
                    let ext = fb.ins().sextend(types::I32, r);
                    write_reg(&mut fb, *dst, ext);
                }
                JitConvKind::IntToChar => {
                    let v = read_reg(&mut fb, *src, pc);
                    let masked = fb.ins().band_imm(v, 0xffff);
                    write_reg(&mut fb, *dst, masked);
                }
                JitConvKind::IntToShort => {
                    let v = read_reg(&mut fb, *src, pc);
                    let r = fb.ins().ireduce(types::I16, v);
                    let ext = fb.ins().sextend(types::I32, r);
                    write_reg(&mut fb, *dst, ext);
                }
            },
            ObjectOp::BinOpLit {
                dst,
                src,
                literal,
                kind,
                ..
            } => {
                let lhs = read_reg(&mut fb, *src, pc);
                let lit = fb.ins().iconst(types::I32, i64::from(*literal));
                let value = match kind {
                    JitLitOp::Add => fb.ins().iadd(lhs, lit),
                    JitLitOp::RSub => fb.ins().isub(lit, lhs),
                    JitLitOp::Mul => fb.ins().imul(lhs, lit),
                    JitLitOp::And => fb.ins().band(lhs, lit),
                    JitLitOp::Or => fb.ins().bor(lhs, lit),
                    JitLitOp::Xor => fb.ins().bxor(lhs, lit),
                    JitLitOp::Shl => {
                        let amount = fb.ins().iconst(types::I32, i64::from(*literal & 0x1f));
                        fb.ins().ishl(lhs, amount)
                    }
                    JitLitOp::Shr => {
                        let amount = fb.ins().iconst(types::I32, i64::from(*literal & 0x1f));
                        fb.ins().sshr(lhs, amount)
                    }
                    JitLitOp::Ushr => {
                        let amount = fb.ins().iconst(types::I32, i64::from(*literal & 0x1f));
                        fb.ins().ushr(lhs, amount)
                    }
                };
                write_reg(&mut fb, *dst, value);
            }
            ObjectOp::UnOp { dst, src, kind, .. } => {
                let operand = read_reg(&mut fb, *src, pc);
                let value = match kind {
                    JitUnOp::Neg => fb.ins().ineg(operand),
                    JitUnOp::Not => fb.ins().bnot(operand),
                };
                write_reg(&mut fb, *dst, value);
            }
            ObjectOp::IgetInt {
                dst,
                obj,
                field_index,
                ..
            } => {
                let d = reg(&mut fb, *dst);
                let o = reg(&mut fb, *obj);
                let f = fb.ins().iconst(types::I32, i64::from(*field_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(iget_ref, &[env, d, o, f, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
                reload(&mut fb, *dst);
            }
            ObjectOp::ArrayGet {
                dst, array, index, ..
            } => {
                let d = reg(&mut fb, *dst);
                let a = reg(&mut fb, *array);
                let i = reg(&mut fb, *index);
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(aget_ref, &[env, d, a, i, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::IgetObject {
                dst,
                obj,
                field_index,
                ..
            } => {
                let d = reg(&mut fb, *dst);
                let o = reg(&mut fb, *obj);
                let f = fb.ins().iconst(types::I32, i64::from(*field_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(iget_obj_ref, &[env, d, o, f, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::IputField {
                val,
                obj,
                field_index,
                ..
            } => {
                let v = reg(&mut fb, *val);
                let o = reg(&mut fb, *obj);
                let f = fb.ins().iconst(types::I32, i64::from(*field_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(iput_ref, &[env, v, o, f, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::SgetInt {
                dst, field_index, ..
            } => {
                let d = reg(&mut fb, *dst);
                let f = fb.ins().iconst(types::I32, i64::from(*field_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(sget_int_ref, &[env, d, f, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
                reload(&mut fb, *dst);
            }
            ObjectOp::SgetObject {
                dst, field_index, ..
            } => {
                let d = reg(&mut fb, *dst);
                let f = fb.ins().iconst(types::I32, i64::from(*field_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(sget_obj_ref, &[env, d, f, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::NewInstance {
                dst, type_index, ..
            } => {
                let d = reg(&mut fb, *dst);
                let t = fb.ins().iconst(types::I32, i64::from(*type_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(new_instance_ref, &[env, d, t, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::ConstString {
                dst, string_index, ..
            } => {
                let d = reg(&mut fb, *dst);
                let sidx = fb.ins().iconst(types::I32, i64::from(*string_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(const_string_ref, &[env, d, sidx, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::ConstClass {
                dst, type_index, ..
            } => {
                let d = reg(&mut fb, *dst);
                let t = fb.ins().iconst(types::I32, i64::from(*type_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(const_class_ref, &[env, d, t, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::InstanceOf {
                dst,
                reg: value_reg,
                type_index,
                ..
            } => {
                let d = reg(&mut fb, *dst);
                let r = reg(&mut fb, *value_reg);
                let t = fb.ins().iconst(types::I32, i64::from(*type_index));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(instance_of_ref, &[env, d, r, t, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
                reload(&mut fb, *dst);
            }
            ObjectOp::Move { dst, src } => {
                let d = reg(&mut fb, *dst);
                let s_ = reg(&mut fb, *src);
                fb.ins().call(move_ref, &[env, d, s_]);
            }
            ObjectOp::Invoke { .. } => {
                flush_all(&mut fb);
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(invoke_ref, &[env, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::MoveResult { dst, wide, .. } => {
                let d = reg(&mut fb, *dst);
                let w = fb.ins().iconst(types::I32, i64::from(*wide));
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(move_result_ref, &[env, d, w, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
            }
            ObjectOp::Goto { target } => {
                fb.ins().jump(op_block[target], &[]);
                current_terminated = true;
            }
            ObjectOp::IfCmp {
                a, b, cmp, target, ..
            } => {
                let lhs = read_reg(&mut fb, *a, pc);
                let rhs = read_reg(&mut fb, *b, pc);
                let cond = fb.ins().icmp(int_cc(*cmp), lhs, rhs);
                let fallthrough = op_block.get(&(index + 1))?;
                fb.ins()
                    .brif(cond, op_block[target], &[], *fallthrough, &[]);
                current_terminated = true;
            }
            ObjectOp::IfZero { a, cmp, target, .. } => {
                let lhs = read_reg(&mut fb, *a, pc);
                let zero = fb.ins().iconst(types::I32, 0);
                let cond = fb.ins().icmp(int_cc(*cmp), lhs, zero);
                let fallthrough = op_block.get(&(index + 1))?;
                fb.ins()
                    .brif(cond, op_block[target], &[], *fallthrough, &[]);
                current_terminated = true;
            }
            ObjectOp::Bail { .. } => {
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(bail_ref, &[env, pcv]);
                let status = fb.inst_results(call)[0];
                fb.ins().jump(exit_block, &[status.into()]);
                current_terminated = true;
            }
            ObjectOp::Return { src, .. } => {
                flush_one(&mut fb, *src);
                let s = reg(&mut fb, *src);
                let pcv = fb.ins().iconst(types::I32, i64::from(pc));
                let call = fb.ins().call(return_int_ref, &[env, s, pcv]);
                let status = fb.inst_results(call)[0];
                check_status(&mut fb, status, exit_block);
                let zero = fb.ins().iconst(types::I32, 0);
                fb.ins().return_(&[zero]);
                current_terminated = true;
            }
            ObjectOp::ReturnVoid => {
                fb.ins().call(return_void_ref, &[env]);
                let zero = fb.ins().iconst(types::I32, 0);
                fb.ins().return_(&[zero]);
                current_terminated = true;
            }
            ObjectOp::ReturnObject { src } => {
                let s = reg(&mut fb, *src);
                fb.ins().call(return_object_ref, &[env, s]);
                let zero = fb.ins().iconst(types::I32, 0);
                fb.ins().return_(&[zero]);
                current_terminated = true;
            }
        }
    }

    if !current_terminated {
        return None;
    }

    fb.switch_to_block(exit_block);
    flush_all(&mut fb);
    fb.ins().return_(&[exit_status]);
    fb.seal_block(exit_block);
    for block in op_block.values() {
        fb.seal_block(*block);
    }
    fb.finalize();

    let func_id = module
        .declare_function("object_method", Linkage::Export, &ctx.func.signature)
        .ok()?;
    module.define_function(func_id, &mut ctx).ok()?;
    module.clear_context(&mut ctx);
    module.finalize_definitions().ok()?;

    let entry = module.get_finalized_function(func_id);
    Some(CompiledObjectMethod {
        _module: module,
        entry,
    })
}

pub fn decode_object_subset(instructions: &[u16]) -> Option<Vec<ObjectOp>> {
    let mut ops = Vec::new();
    let mut op_start_unit = Vec::new();
    let mut pending_targets: Vec<(usize, usize)> = Vec::new();
    let mut pc = 0usize;
    while pc < instructions.len() {
        let insn = instructions[pc];
        let opcode = (insn & 0x00ff) as u8;
        op_start_unit.push(pc);
        match opcode {
            0x12 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let literal = sign_extend_4(((insn >> 12) & 0x0f) as u8);
                ops.push(ObjectOp::Const { dst, literal });
                pc += 1;
            }
            0x13 => {
                let dst = u16::from((insn >> 8) as u8);
                let literal = i32::from(*instructions.get(pc + 1)? as i16);
                ops.push(ObjectOp::Const { dst, literal });
                pc += 2;
            }
            0x90 | 0x91 | 0x92 | 0x95 | 0x96 | 0x97 | 0x98 | 0x99 | 0x9a => {
                let dst = u16::from((insn >> 8) as u8);
                let regs = *instructions.get(pc + 1)?;
                let a = regs & 0x00ff;
                let b = regs >> 8;
                let kind = int_binop_kind(opcode)?;
                ops.push(ObjectOp::BinOp { dst, a, b, kind });
                pc += 2;
            }
            0x01 | 0x07 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let src = u16::from((insn >> 12) & 0x0f);
                ops.push(ObjectOp::Move { dst, src });
                pc += 1;
            }
            0x02 | 0x08 => {
                let dst = u16::from((insn >> 8) as u8);
                let src = *instructions.get(pc + 1)?;
                ops.push(ObjectOp::Move { dst, src });
                pc += 2;
            }
            0x20 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let reg = u16::from((insn >> 12) & 0x0f);
                let type_index = u32::from(*instructions.get(pc + 1)?);
                ops.push(ObjectOp::InstanceOf {
                    dst,
                    reg,
                    type_index,
                    pc: u32::try_from(pc).ok()?,
                });
                pc += 2;
            }
            0x52 | 0x55 | 0x56 | 0x57 | 0x58 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let obj = u16::from((insn >> 12) & 0x0f);
                let field_index = u32::from(*instructions.get(pc + 1)?);
                ops.push(ObjectOp::IgetInt {
                    dst,
                    obj,
                    field_index,
                    pc: u32::try_from(pc).ok()?,
                });
                pc += 2;
            }
            0x54 => {
                let dst = u16::from((insn >> 8) & 0x0f);
                let obj = u16::from((insn >> 12) & 0x0f);
                let field_index = u32::from(*instructions.get(pc + 1)?);
                ops.push(ObjectOp::IgetObject {
                    dst,
                    obj,
                    field_index,
                    pc: u32::try_from(pc).ok()?,
                });
                pc += 2;
            }
            0x28 => {
                let offset = (insn >> 8) as u8 as i8 as isize;
                let target = pc.checked_add_signed(offset)?;
                pending_targets.push((ops.len(), target));
                ops.push(ObjectOp::Goto { target: 0 });
                pc += 1;
            }
            // cite: art interpreter_switch_impl-inl.h#IF_LT, IF_GE, IF_GT, IF_LE
            0x34..=0x37 => {
                let a = u16::from((insn >> 8) & 0x0f);
                let b = u16::from((insn >> 12) & 0x0f);
                let offset = *instructions.get(pc + 1)? as i16 as isize;
                let target = pc.checked_add_signed(offset)?;
                let cmp = compare_branch_cmp(opcode)?;
                pending_targets.push((ops.len(), target));
                ops.push(ObjectOp::IfCmp {
                    a,
                    b,
                    cmp,
                    target: 0,
                    pc: pc as u32,
                });
                pc += 2;
            }
            0x3a..=0x3d => {
                let a = u16::from((insn >> 8) as u8);
                let offset = *instructions.get(pc + 1)? as i16 as isize;
                let target = pc.checked_add_signed(offset)?;
                let cmp = zero_branch_cmp(opcode)?;
                pending_targets.push((ops.len(), target));
                ops.push(ObjectOp::IfZero {
                    a,
                    cmp,
                    target: 0,
                    pc: pc as u32,
                });
                pc += 2;
            }
            0x60..=0x66 => return None,
            0x59 | 0x5b | 0x5c | 0x5d | 0x5e | 0x5f => {
                let val = u16::from((insn >> 8) & 0x0f);
                let obj = u16::from((insn >> 12) & 0x0f);
                let field_index = u32::from(*instructions.get(pc + 1)?);
                ops.push(ObjectOp::IputField {
                    val,
                    obj,
                    field_index,
                    pc: u32::try_from(pc).ok()?,
                });
                pc += 2;
            }
            0x0f => {
                let src = u16::from((insn >> 8) as u8);
                ops.push(ObjectOp::Return {
                    src,
                    pc: u32::try_from(pc).ok()?,
                });
                pc += 1;
            }
            0x0e => {
                ops.push(ObjectOp::ReturnVoid);
                pc += 1;
            }
            0x11 => {
                let src = u16::from((insn >> 8) as u8);
                ops.push(ObjectOp::ReturnObject { src });
                pc += 1;
            }
            0x0a | 0x0b | 0x0c => {
                let dst = u16::from((insn >> 8) as u8);
                ops.push(ObjectOp::MoveResult {
                    dst,
                    wide: opcode == 0x0b,
                    pc: u32::try_from(pc).ok()?,
                });
                pc += 1;
            }
            0x6e..=0x72 | 0x74..=0x78 => {
                let invoke_pc = u32::try_from(pc).ok()?;
                if pc + 2 >= instructions.len() {
                    return None;
                }
                ops.push(ObjectOp::Invoke { pc: invoke_pc });
                pc += 3;
            }
            _ => return None,
        }
    }

    let unit_to_op: HashMap<usize, usize> = op_start_unit
        .iter()
        .enumerate()
        .map(|(op_index, unit)| (*unit, op_index))
        .collect();
    for (op_index, target_unit) in pending_targets {
        let resolved = *unit_to_op.get(&target_unit)?;
        match &mut ops[op_index] {
            ObjectOp::Goto { target }
            | ObjectOp::IfCmp { target, .. }
            | ObjectOp::IfZero { target, .. } => *target = resolved,
            _ => return None,
        }
    }
    Some(ops)
}

// cite: art jit.cc#CompileMethod
thread_local! {
    static LAST_COMPILE: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
}

// cite: art jit.cc#CompileMethod
fn compile_throttled() -> bool {
    const MIN_INTERVAL_MS: u128 = 12;
    LAST_COMPILE.with(|last| match last.get() {
        Some(when) if when.elapsed().as_millis() < MIN_INTERVAL_MS => true,
        _ => {
            last.set(Some(std::time::Instant::now()));
            false
        }
    })
}

// cite: art stack_map.h#StackMap
// cite: art code_generator.cc#RecordPcInfo
#[derive(Default)]
pub struct CompiledMeta {
    pub deopt: HashMap<u32, Vec<crate::dex3oat::deopt::ResumePoint>>,
}

enum ObjectJitEntry {
    Counting(u32),
    Compiled(std::rc::Rc<CompiledObjectMethod>, std::rc::Rc<CompiledMeta>),
    Unsupported,
}

impl JitCache {
    pub fn advance_object(
        &mut self,
        dex_file_index: usize,
        code_offset: u32,
        instructions: &[u16],
        first_param_reg: u16,
        abi: ObjectMethodAbi,
        dex3oat_ops: impl FnOnce() -> Option<(Vec<ObjectOp>, CompiledMeta)>,
    ) -> bool {
        let key = (dex_file_index, code_offset);
        match self.object_entries.get_mut(&key) {
            Some(ObjectJitEntry::Unsupported) => false,
            Some(ObjectJitEntry::Compiled(..)) => true,
            Some(ObjectJitEntry::Counting(count)) => {
                *count += 1;
                if *count < HOT_METHOD_THRESHOLD {
                    return false;
                }
                // cite: art jit.cc#CompileMethod
                if std::env::var_os("WINEDROID_DEX3OAT_GRAPH").is_some() && compile_throttled() {
                    return false;
                }
                match dex3oat_ops()
                    .or_else(|| {
                        decode_object_subset(instructions).map(|ops| (ops, CompiledMeta::default()))
                    })
                    .and_then(|(ops, deopt_map)| {
                        compile_object_method(&ops, first_param_reg, abi).map(|c| (c, deopt_map))
                    }) {
                    Some((compiled, meta)) => {
                        trace::event(
                            "jit",
                            format!(
                                "compiled object method dex={dex_file_index} offset={code_offset}"
                            ),
                        );
                        self.object_entries.insert(
                            key,
                            ObjectJitEntry::Compiled(
                                std::rc::Rc::new(compiled),
                                std::rc::Rc::new(meta),
                            ),
                        );
                        true
                    }
                    None => {
                        self.object_entries.insert(key, ObjectJitEntry::Unsupported);
                        false
                    }
                }
            }
            None => {
                self.object_entries.insert(key, ObjectJitEntry::Counting(1));
                false
            }
        }
    }

    #[cfg(test)]
    pub fn run_object(
        &mut self,
        dex_file_index: usize,
        code_offset: u32,
        instructions: &[u16],
        first_param_reg: u16,
        abi: ObjectMethodAbi,
    ) -> Option<std::rc::Rc<CompiledObjectMethod>> {
        if !self.advance_object(
            dex_file_index,
            code_offset,
            instructions,
            first_param_reg,
            abi,
            || None,
        ) {
            return None;
        }
        self.peek_object(dex_file_index, code_offset)
            .map(|(compiled, _)| compiled)
    }

    pub fn peek_object(
        &self,
        dex_file_index: usize,
        code_offset: u32,
    ) -> Option<(std::rc::Rc<CompiledObjectMethod>, std::rc::Rc<CompiledMeta>)> {
        match self.object_entries.get(&(dex_file_index, code_offset)) {
            Some(ObjectJitEntry::Compiled(compiled, meta)) => {
                Some((compiled.clone(), meta.clone()))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bytecode() -> Vec<u16> {
        vec![0x5012, 0x0113, 0x0025, 0x0090, 0x0100, 0x000f]
    }

    #[test]
    fn jit_cache_warms_up_then_compiles_and_dispatches() {
        let mut cache = JitCache::new();
        let code = sample_bytecode();
        for _ in 0..(HOT_METHOD_THRESHOLD - 1) {
            assert_eq!(cache.run(0, 100, &code, 2, &[0, 0]), None);
        }
        assert_eq!(cache.run(0, 100, &code, 2, &[0, 0]), Some(42));
        assert_eq!(cache.run(0, 100, &code, 2, &[0, 0]), Some(42));
    }

    #[test]
    fn jit_cache_sticky_rejects_unsupported_methods() {
        let mut cache = JitCache::new();
        let code = vec![0x000f, 0x006e];
        for _ in 0..(HOT_METHOD_THRESHOLD + 3) {
            assert_eq!(cache.run(0, 200, &code, 1, &[0]), None);
        }
    }

    #[test]
    fn decodes_and_runs_real_dex_bytecode_end_to_end() {
        let bytecode: Vec<u16> = vec![0x5012, 0x0113, 0x0025, 0x0090, 0x0100, 0x000f];
        let ops = decode_integer_subset(&bytecode).expect("subset should decode");
        assert_eq!(
            ops,
            vec![
                JitOp::Const { dst: 0, literal: 5 },
                JitOp::Const {
                    dst: 1,
                    literal: 37
                },
                JitOp::BinOp {
                    dst: 0,
                    a: 0,
                    b: 1,
                    kind: JitBinOp::Add,
                },
                JitOp::Return { src: 0 },
            ]
        );
        let compiled = compile(&ops, 2).expect("decoded subset should compile");
        assert_eq!(compiled.call(&[0, 0]), 42);
    }

    #[test]
    fn decode_rejects_unsupported_opcode() {
        assert!(decode_integer_subset(&[0x000f, 0x006e]).is_none());
    }

    #[test]
    fn decodes_branches_and_resolves_targets_to_op_indices() {
        let bytecode: Vec<u16> = vec![0x003d, 0x0004, 0x0191, 0x0001, 0x000f];
        let ops = decode_integer_subset(&bytecode).expect("should decode");
        assert_eq!(
            ops[0],
            JitOp::IfZero {
                a: 0,
                cmp: JitCmp::Le,
                target: 2,
            }
        );
        assert_eq!(ops[2], JitOp::Return { src: 0 });
    }

    #[test]
    fn decode_rejects_branch_into_instruction_middle() {
        let bytecode: Vec<u16> = vec![0x0013, 0x0007, 0xff28];
        assert!(decode_integer_subset(&bytecode).is_none());
    }

    #[test]
    fn compiles_and_runs_integer_arithmetic() {
        let ops = vec![
            JitOp::Const { dst: 0, literal: 5 },
            JitOp::Const {
                dst: 1,
                literal: 37,
            },
            JitOp::BinOp {
                dst: 0,
                a: 0,
                b: 1,
                kind: JitBinOp::Add,
            },
            JitOp::Return { src: 0 },
        ];
        let compiled = compile(&ops, 2).expect("subset should compile");
        assert_eq!(compiled.call(&[0, 0]), 42);
    }

    #[test]
    fn reads_input_registers_and_multiplies() {
        let ops = vec![
            JitOp::BinOp {
                dst: 2,
                a: 0,
                b: 1,
                kind: JitBinOp::Mul,
            },
            JitOp::Return { src: 2 },
        ];
        let compiled = compile(&ops, 3).expect("subset should compile");
        assert_eq!(compiled.call(&[6, 7, 0]), 42);
        assert_eq!(compiled.call(&[9, 9, 0]), 81);
    }

    #[test]
    fn rejects_out_of_range_registers() {
        let ops = vec![JitOp::Return { src: 5 }];
        assert!(compile(&ops, 2).is_none());
    }

    #[test]
    fn rejects_stream_without_return() {
        let ops = vec![JitOp::Const { dst: 0, literal: 1 }];
        assert!(compile(&ops, 1).is_none());
    }

    #[test]
    fn compiles_and_runs_a_loop_with_conditional_branch() {
        let ops = vec![
            JitOp::Const { dst: 1, literal: 0 },
            JitOp::IfZero {
                a: 0,
                cmp: JitCmp::Le,
                target: 5,
            },
            JitOp::BinOp {
                dst: 1,
                a: 1,
                b: 0,
                kind: JitBinOp::Add,
            },
            JitOp::BinOp {
                dst: 0,
                a: 0,
                b: 2,
                kind: JitBinOp::Sub,
            },
            JitOp::Goto { target: 1 },
            JitOp::Return { src: 1 },
        ];
        let compiled = compile(&ops, 3).expect("loop should compile");
        assert_eq!(compiled.call(&[5, 0, 1]), 15);
        assert_eq!(compiled.call(&[10, 0, 1]), 55);
        assert_eq!(compiled.call(&[0, 0, 1]), 0);
    }

    #[test]
    fn compiles_forward_conditional_branch() {
        let ops = vec![
            JitOp::IfCmp {
                a: 0,
                b: 1,
                cmp: JitCmp::Ge,
                target: 3,
            },
            JitOp::Move { dst: 2, src: 1 },
            JitOp::Goto { target: 4 },
            JitOp::Move { dst: 2, src: 0 },
            JitOp::Return { src: 2 },
        ];
        let compiled = compile(&ops, 3).expect("branch should compile");
        assert_eq!(compiled.call(&[3, 8, 0]), 8);
        assert_eq!(compiled.call(&[9, 4, 0]), 9);
        assert_eq!(compiled.call(&[5, 5, 0]), 5);
    }

    #[test]
    fn rejects_branch_target_out_of_range() {
        let ops = vec![JitOp::Goto { target: 99 }, JitOp::Return { src: 0 }];
        assert!(compile(&ops, 1).is_none());
    }

    #[test]
    fn compiles_literal_and_unary_integer_ops() {
        let ops = vec![
            JitOp::BinOpLit {
                dst: 1,
                src: 0,
                literal: 10,
                kind: JitLitOp::Add,
            },
            JitOp::BinOpLit {
                dst: 1,
                src: 1,
                literal: 100,
                kind: JitLitOp::RSub,
            },
            JitOp::BinOpLit {
                dst: 1,
                src: 1,
                literal: 3,
                kind: JitLitOp::Mul,
            },
            JitOp::UnOp {
                dst: 1,
                src: 1,
                kind: JitUnOp::Neg,
            },
            JitOp::UnOp {
                dst: 1,
                src: 1,
                kind: JitUnOp::Not,
            },
            JitOp::Return { src: 1 },
        ];
        let compiled = compile(&ops, 2).expect("should compile");
        let expected = {
            let mut v: i32 = 5;
            v = v.wrapping_add(10);
            v = 100i32.wrapping_sub(v);
            v = v.wrapping_mul(3);
            v = v.wrapping_neg();
            v = !v;
            v
        };
        assert_eq!(compiled.call(&[5, 0]), expected);
        assert_eq!(expected, 254);
    }

    // cite: art dex_instruction_list.h#SHL_INT_LIT8, SHR_INT_LIT8, USHR_INT_LIT8
    #[test]
    fn art_int_lit8_shift_ops_mask_the_literal_amount() {
        let decoded = decode_integer_subset(&[0x01e0, 0x2800, 0x010f]).expect("decode");
        assert_eq!(
            decoded,
            vec![
                JitOp::BinOpLit {
                    dst: 1,
                    src: 0,
                    literal: 40,
                    kind: JitLitOp::Shl,
                },
                JitOp::Return { src: 1 },
            ]
        );
        for (kind, input, literal, expected) in [
            (JitLitOp::Shl, 1, 40, 256),
            (JitLitOp::Shr, -256, 36, -16),
            (JitLitOp::Ushr, -1, 33, i32::MAX),
        ] {
            let ops = vec![
                JitOp::BinOpLit {
                    dst: 1,
                    src: 0,
                    literal,
                    kind,
                },
                JitOp::Return { src: 1 },
            ];
            let compiled = compile(&ops, 2).expect("compile");
            assert_eq!(compiled.call(&[input, 0]), expected);
        }
    }

    #[test]
    fn compiles_bitwise_and_shift_ops() {
        let and = |a, b| a & b;
        for &(kind, cl) in &[
            (JitBinOp::And, and as fn(i32, i32) -> i32),
            (JitBinOp::Or, (|a, b| a | b) as fn(i32, i32) -> i32),
            (JitBinOp::Xor, (|a, b| a ^ b) as fn(i32, i32) -> i32),
        ] {
            let ops = vec![
                JitOp::BinOp {
                    dst: 2,
                    a: 0,
                    b: 1,
                    kind,
                },
                JitOp::Return { src: 2 },
            ];
            let compiled = compile(&ops, 3).unwrap();
            assert_eq!(compiled.call(&[0xF0, 0x0F, 0]), cl(0xF0, 0x0F));
        }
        for &(kind, cl) in &[
            (
                JitBinOp::Shl,
                (|a: i32, s: i32| a.wrapping_shl((s & 0x1f) as u32)) as fn(i32, i32) -> i32,
            ),
            (
                JitBinOp::Shr,
                (|a: i32, s: i32| a.wrapping_shr((s & 0x1f) as u32)) as fn(i32, i32) -> i32,
            ),
            (
                JitBinOp::Ushr,
                (|a: i32, s: i32| ((a as u32).wrapping_shr((s & 0x1f) as u32)) as i32)
                    as fn(i32, i32) -> i32,
            ),
        ] {
            let ops = vec![
                JitOp::BinOp {
                    dst: 2,
                    a: 0,
                    b: 1,
                    kind,
                },
                JitOp::Return { src: 2 },
            ];
            let compiled = compile(&ops, 3).unwrap();
            assert_eq!(compiled.call(&[-16, 3, 0]), cl(-16, 3));
            assert_eq!(compiled.call(&[-16, 33, 0]), cl(-16, 33));
        }
    }

    extern "C" fn add_ctx_counter(ctx: *mut std::ffi::c_void, arg: i32) -> i32 {
        let counter = unsafe { &mut *(ctx as *mut i32) };
        *counter += 1;
        *counter + arg
    }

    #[repr(C)]
    struct MockEnv {
        regs: [i32; 8],
        fields: std::collections::HashMap<u32, i32>,
        returned: Option<i32>,
        returned_void: bool,
        handoff: Option<u32>,
        last_result: Option<i32>,
        invoked_at: Vec<u32>,
        park_at: Option<u32>,
        regs_at_invoke: Option<[i32; 8]>,
    }

    fn mock_env(regs: [i32; 8], fields: &[(u32, i32)]) -> MockEnv {
        MockEnv {
            regs,
            fields: fields.iter().copied().collect(),
            returned: None,
            returned_void: false,
            handoff: None,
            last_result: None,
            invoked_at: Vec::new(),
            park_at: None,
            regs_at_invoke: None,
        }
    }

    extern "C" fn mock_get_int(env: *mut std::ffi::c_void, reg: u32, _pc: u32) -> i64 {
        let env = unsafe { &*(env as *const MockEnv) };
        i64::from(env.regs[reg as usize] as u32)
    }
    extern "C" fn mock_set_int(env: *mut std::ffi::c_void, reg: u32, val: i32) {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.regs[reg as usize] = val;
    }
    extern "C" fn mock_get_long(
        env: *mut std::ffi::c_void,
        reg: u32,
        _pc: u32,
        status: *mut i32,
    ) -> i64 {
        let env = unsafe { &*(env as *const MockEnv) };
        unsafe { *status = 0 };
        i64::from(env.regs[reg as usize])
    }
    extern "C" fn mock_set_long(env: *mut std::ffi::c_void, reg: u32, val: i64) {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.regs[reg as usize] = val as i32;
    }
    extern "C" fn mock_get_float_bits(env: *mut std::ffi::c_void, reg: u32, _pc: u32) -> i64 {
        let env = unsafe { &*(env as *const MockEnv) };
        i64::from(env.regs[reg as usize] as u32)
    }
    extern "C" fn mock_set_float_bits(env: *mut std::ffi::c_void, reg: u32, bits: i32) {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.regs[reg as usize] = bits;
    }
    extern "C" fn mock_aget(
        env: *mut std::ffi::c_void,
        dst: u32,
        _array: u32,
        _index: u32,
        _pc: u32,
    ) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.regs[dst as usize] = 0;
        0
    }
    extern "C" fn mock_iget_int(
        env: *mut std::ffi::c_void,
        dst: u32,
        _obj: u32,
        field_index: u32,
        _pc: u32,
    ) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        match env.fields.get(&field_index) {
            Some(v) => {
                env.regs[dst as usize] = *v;
                0
            }
            None => 1,
        }
    }
    extern "C" fn mock_move(env: *mut std::ffi::c_void, dst: u32, src: u32) {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.regs[dst as usize] = env.regs[src as usize];
    }
    extern "C" fn mock_iget_object(
        env: *mut std::ffi::c_void,
        dst: u32,
        obj: u32,
        field_index: u32,
        pc: u32,
    ) -> i32 {
        mock_iget_int(env, dst, obj, field_index, pc)
    }
    extern "C" fn mock_iput_field(
        env: *mut std::ffi::c_void,
        val: u32,
        _obj: u32,
        field_index: u32,
        _pc: u32,
    ) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.fields.insert(field_index, env.regs[val as usize]);
        0
    }
    extern "C" fn mock_sget(
        env: *mut std::ffi::c_void,
        dst: u32,
        field_index: u32,
        _pc: u32,
    ) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        match env.fields.get(&field_index) {
            Some(v) => {
                env.regs[dst as usize] = *v;
                0
            }
            None => 1,
        }
    }
    extern "C" fn mock_invoke(env: *mut std::ffi::c_void, pc: u32) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.invoked_at.push(pc);
        env.regs_at_invoke = Some(env.regs);
        env.handoff = Some(pc);
        1
    }
    extern "C" fn mock_move_result(
        env: *mut std::ffi::c_void,
        dst: u32,
        _wide: u32,
        _pc: u32,
    ) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        match env.last_result.take() {
            Some(value) => {
                env.regs[dst as usize] = value;
                0
            }
            None => 1,
        }
    }
    extern "C" fn mock_return_int(env: *mut std::ffi::c_void, src: u32, _pc: u32) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.returned = Some(env.regs[src as usize]);
        0
    }
    extern "C" fn mock_return_void(env: *mut std::ffi::c_void) {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.returned_void = true;
    }
    extern "C" fn mock_return_object(env: *mut std::ffi::c_void, src: u32) {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.returned = Some(env.regs[src as usize]);
    }
    extern "C" fn mock_new_like(env: *mut std::ffi::c_void, dst: u32, index: u32, _pc: u32) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.regs[dst as usize] = index as i32;
        0
    }
    extern "C" fn mock_instance_of(
        env: *mut std::ffi::c_void,
        dst: u32,
        reg: u32,
        type_index: u32,
        _pc: u32,
    ) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.regs[dst as usize] = i32::from(env.regs[reg as usize] == type_index as i32);
        0
    }
    extern "C" fn mock_bail(env: *mut std::ffi::c_void, pc: u32) -> i32 {
        let env = unsafe { &mut *(env as *mut MockEnv) };
        env.handoff = Some(pc);
        1
    }

    fn mock_abi() -> ObjectMethodAbi {
        ObjectMethodAbi {
            get_int: mock_get_int,
            set_int: mock_set_int,
            get_long: mock_get_long,
            set_long: mock_set_long,
            get_float_bits: mock_get_float_bits,
            set_float_bits: mock_set_float_bits,
            iget_int: mock_iget_int,
            aget: mock_aget,
            move_reg: mock_move,
            iget_object: mock_iget_object,
            iput_field: mock_iput_field,
            sget_int: mock_sget,
            sget_object: mock_sget,
            invoke: mock_invoke,
            move_result: mock_move_result,
            return_int: mock_return_int,
            return_void: mock_return_void,
            return_object: mock_return_object,
            new_instance: mock_new_like,
            const_string: mock_new_like,
            const_class: mock_new_like,
            instance_of: mock_instance_of,
            bail: mock_bail,
        }
    }

    #[test]
    fn object_method_with_field_write() {
        let ops = vec![
            ObjectOp::IputField {
                val: 0,
                obj: 1,
                field_index: 5,
                pc: 0,
            },
            ObjectOp::Return { src: 0, pc: 0 },
        ];
        let abi = mock_abi();
        let compiled = compile_object_method(&ops, 100, abi).expect("should compile");
        let mut env = mock_env([99, 0, 0, 0, 0, 0, 0, 0], &[]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 0);
        assert_eq!(env.returned, Some(99));
        assert_eq!(env.fields.get(&5), Some(&99));
    }

    #[test]
    fn object_decode_accepts_iput() {
        assert!(decode_object_subset(&[0x1059, 0x0005, 0x000f]).is_some());
        assert!(decode_object_subset(&[0x105a, 0x0005, 0x000f]).is_none());
    }

    // cite: art nterp.cc#NterpGetStaticField
    #[test]
    fn object_decode_rejects_static_get_that_can_run_class_initialization() {
        for opcode in 0x60..=0x66 {
            assert!(decode_object_subset(&[opcode, 0x0005, 0x000f]).is_none());
        }
    }

    #[test]
    fn object_method_with_control_flow() {
        let ops = vec![
            ObjectOp::IgetInt {
                dst: 0,
                obj: 3,
                field_index: 1,
                pc: 0,
            },
            ObjectOp::IgetInt {
                dst: 1,
                obj: 3,
                field_index: 2,
                pc: 0,
            },
            ObjectOp::IfCmp {
                a: 0,
                b: 1,
                cmp: JitCmp::Ge,
                target: 4,
                pc: 0,
            },
            ObjectOp::Return { src: 1, pc: 0 },
            ObjectOp::Return { src: 0, pc: 0 },
        ];
        let abi = mock_abi();
        let compiled = compile_object_method(&ops, 100, abi).expect("should compile");
        let mut env = mock_env([0; 8], &[(1, 7), (2, 12)]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 0);
        assert_eq!(env.returned, Some(12));
        let mut env2 = mock_env([0; 8], &[(1, 20), (2, 3)]);
        let p2 = &mut env2 as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p2) }, 0);
        assert_eq!(env2.returned, Some(20));
    }

    #[test]
    fn long_stability_rejects_register_reused_as_object_before_wide_result() {
        let ops = vec![
            ObjectOp::SgetObject {
                dst: 0,
                field_index: 0x36fe,
                pc: 0,
            },
            ObjectOp::Invoke { pc: 2 },
            ObjectOp::MoveResult {
                dst: 0,
                wide: true,
                pc: 5,
            },
            ObjectOp::LongConst {
                dst: 2,
                value: 0x0fff_ffff_ffff_ffff,
            },
            ObjectOp::LongBinOp {
                dst: 0,
                a: 0,
                b: 2,
                kind: JitLongBinOp::And,
            },
            ObjectOp::Return { src: 0, pc: 12 },
        ];
        let stable = object_int_stable_registers(&ops, 4);
        let long_stable = object_long_stable_registers(&ops, &stable, 4);
        assert!(!long_stable[0]);
        assert!(!long_stable[1]);
        assert!(long_stable[2]);
        let compiled = compile_object_method(&ops, 4, mock_abi()).expect("compile");
        let mut env = mock_env([0; 8], &[(0x36fe, 77)]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 1);
        assert_eq!(env.regs_at_invoke.expect("invoke ran")[0], 77);
    }

    #[test]
    fn shadowed_registers_are_flushed_to_frame_before_an_invoke() {
        let ops = vec![
            ObjectOp::Const { dst: 0, literal: 7 },
            ObjectOp::BinOp {
                dst: 0,
                a: 0,
                b: 0,
                kind: JitBinOp::Add,
            },
            ObjectOp::Invoke { pc: 3 },
            ObjectOp::MoveResult {
                dst: 2,
                wide: false,
                pc: 6,
            },
            ObjectOp::Return { src: 2, pc: 7 },
        ];
        assert!(object_int_stable_registers(&ops, 100)[0]);
        assert!(!object_int_stable_registers(&ops, 100)[2]);
        let compiled = compile_object_method(&ops, 0, mock_abi()).expect("compile");
        let mut env = mock_env([0; 8], &[]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 1);
        assert_eq!(env.regs_at_invoke.expect("invoke ran")[0], 14);
        assert_eq!(env.handoff, Some(3));
        assert_eq!(env.returned, None);
    }

    // cite: art dex_instruction_list.h#THROW
    #[test]
    fn art_throw_block_bails_only_when_reached() {
        let ops = vec![
            ObjectOp::IfZero {
                a: 0,
                cmp: JitCmp::Eq,
                target: 3,
                pc: 0,
            },
            ObjectOp::Const { dst: 1, literal: 9 },
            ObjectOp::Return { src: 1, pc: 2 },
            ObjectOp::Bail { pc: 8 },
        ];
        let compiled = compile_object_method(&ops, 0, mock_abi()).expect("compile");

        let mut normal = mock_env([1, 0, 0, 0, 0, 0, 0, 0], &[]);
        let normal_ptr = &mut normal as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(normal_ptr) }, 0);
        assert_eq!(normal.returned, Some(9));
        assert_eq!(normal.handoff, None);

        let mut thrown = mock_env([0, 0, 0, 0, 0, 0, 0, 0], &[]);
        let thrown_ptr = &mut thrown as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(thrown_ptr) }, 1);
        assert_eq!(thrown.returned, None);
        assert_eq!(thrown.handoff, Some(8));
    }

    // cite: art interpreter_switch_impl-inl.h#HandleCmpl, HandleCmpg
    #[test]
    fn object_method_compares_float_with_nan_bias() {
        let cmpl = compile_object_method(
            &[
                ObjectOp::CmpFloat {
                    dst: 0,
                    a: 1,
                    b: 2,
                    bias: JitFloatCmpBias::Less,
                    pc: 0,
                },
                ObjectOp::Return { src: 0, pc: 1 },
            ],
            100,
            mock_abi(),
        )
        .expect("compile");
        let cmpg = compile_object_method(
            &[
                ObjectOp::CmpFloat {
                    dst: 0,
                    a: 1,
                    b: 2,
                    bias: JitFloatCmpBias::Greater,
                    pc: 0,
                },
                ObjectOp::Return { src: 0, pc: 1 },
            ],
            100,
            mock_abi(),
        )
        .expect("compile");

        for (left, right, expected) in [(2.0f32, 3.0f32, -1), (3.0, 2.0, 1), (3.0, 3.0, 0)] {
            let mut env = mock_env(
                [
                    0,
                    left.to_bits() as i32,
                    right.to_bits() as i32,
                    0,
                    0,
                    0,
                    0,
                    0,
                ],
                &[],
            );
            let ptr = &mut env as *mut MockEnv as *mut std::ffi::c_void;
            assert_eq!(unsafe { cmpl.call(ptr) }, 0);
            assert_eq!(env.returned, Some(expected));
        }

        let mut cmpl_nan = mock_env(
            [
                0,
                f32::NAN.to_bits() as i32,
                3.0f32.to_bits() as i32,
                0,
                0,
                0,
                0,
                0,
            ],
            &[],
        );
        let cmpl_nan_ptr = &mut cmpl_nan as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { cmpl.call(cmpl_nan_ptr) }, 0);
        assert_eq!(cmpl_nan.returned, Some(-1));

        let mut cmpg_nan = mock_env(
            [
                0,
                f32::NAN.to_bits() as i32,
                3.0f32.to_bits() as i32,
                0,
                0,
                0,
                0,
                0,
            ],
            &[],
        );
        let cmpg_nan_ptr = &mut cmpg_nan as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { cmpg.call(cmpg_nan_ptr) }, 0);
        assert_eq!(cmpg_nan.returned, Some(1));
    }

    #[test]
    fn shadowed_accumulator_loops_with_an_invoke_each_iteration() {
        let ops = vec![
            ObjectOp::Const { dst: 0, literal: 0 },
            ObjectOp::Const { dst: 1, literal: 3 },
            ObjectOp::IfZero {
                a: 1,
                cmp: JitCmp::Le,
                target: 7,
                pc: 0,
            },
            ObjectOp::Invoke { pc: 6 },
            ObjectOp::BinOp {
                dst: 0,
                a: 0,
                b: 1,
                kind: JitBinOp::Add,
            },
            ObjectOp::BinOp {
                dst: 1,
                a: 1,
                b: 5,
                kind: JitBinOp::Sub,
            },
            ObjectOp::Goto { target: 2 },
            ObjectOp::Return { src: 0, pc: 20 },
        ];
        assert!(object_int_stable_registers(&ops, 5)[0]);
        assert!(object_int_stable_registers(&ops, 5)[1]);
        assert!(!object_int_stable_registers(&ops, 5)[5]);
        let compiled = compile_object_method(&ops, 5, mock_abi()).expect("compile");
        let mut env = mock_env([0, 0, 0, 0, 0, 1, 0, 0], &[]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 1);
        assert_eq!(env.invoked_at, vec![6]);
        assert_eq!(env.returned, None);
    }

    #[test]
    fn object_method_executes_through_an_invoke() {
        let bytecode: Vec<u16> = vec![0x1052, 0x0003, 0x106e, 0x0009, 0x0001, 0x000a, 0x000f];
        let ops = decode_object_subset(&bytecode).expect("should decode");
        assert_eq!(
            ops,
            vec![
                ObjectOp::IgetInt {
                    dst: 0,
                    obj: 1,
                    field_index: 3,
                    pc: 0,
                },
                ObjectOp::Invoke { pc: 2 },
                ObjectOp::MoveResult {
                    dst: 0,
                    wide: false,
                    pc: 5,
                },
                ObjectOp::Return { src: 0, pc: 6 },
            ]
        );
        let compiled = compile_object_method(&ops, 100, mock_abi()).expect("should compile");
        let mut env = mock_env([0; 8], &[(3, 55)]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 1);
        assert_eq!(env.invoked_at, vec![2]);
        assert_eq!(env.regs[0], 55);
        assert_eq!(env.returned, None);
    }

    #[test]
    fn object_method_stops_when_an_invoke_parks() {
        let bytecode: Vec<u16> = vec![0x1052, 0x0003, 0x106e, 0x0009, 0x0001, 0x000a, 0x000f];
        let ops = decode_object_subset(&bytecode).expect("should decode");
        let compiled = compile_object_method(&ops, 100, mock_abi()).expect("should compile");
        let mut env = mock_env([0; 8], &[(3, 55)]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 1);
        assert_eq!(env.handoff, Some(2));
        assert_eq!(env.regs[0], 55);
        assert_eq!(env.returned, None);
    }

    #[test]
    fn object_method_returns_void_and_object() {
        let compiled =
            compile_object_method(&[ObjectOp::ReturnVoid], 100, mock_abi()).expect("compile");
        let mut env = mock_env([0; 8], &[]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 0);
        assert!(env.returned_void);
        let compiled = compile_object_method(&[ObjectOp::ReturnObject { src: 2 }], 100, mock_abi())
            .expect("compile");
        let mut env = mock_env([0, 0, 77, 0, 0, 0, 0, 0], &[]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 0);
        assert_eq!(env.returned, Some(77));
    }

    #[test]
    fn object_decode_accepts_returns_and_invoke_forms() {
        assert_eq!(
            decode_object_subset(&[0x000e]).expect("decode"),
            vec![ObjectOp::ReturnVoid]
        );
        assert_eq!(
            decode_object_subset(&[0x0111]).expect("decode"),
            vec![ObjectOp::ReturnObject { src: 1 }]
        );
        let ops = decode_object_subset(&[0x0012, 0x0077, 0x0004, 0x0000, 0x000e]).expect("decode");
        assert_eq!(ops[1], ObjectOp::Invoke { pc: 1 });
        assert!(decode_object_subset(&[0x006e, 0x0009]).is_none());
    }

    #[test]
    fn object_method_executes_instance_of() {
        let bytecode: Vec<u16> = vec![0x1020, 0x0007, 0x000f];
        let ops = decode_object_subset(&bytecode).expect("decode");
        assert_eq!(
            ops,
            vec![
                ObjectOp::InstanceOf {
                    dst: 0,
                    reg: 1,
                    type_index: 7,
                    pc: 0,
                },
                ObjectOp::Return { src: 0, pc: 2 },
            ]
        );
        let compiled = compile_object_method(&ops, 100, mock_abi()).expect("compile");
        let mut env = mock_env([0, 7, 0, 0, 0, 0, 0, 0], &[]);
        let p = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(p) }, 0);
        assert_eq!(env.returned, Some(1));
    }

    #[test]
    fn run_object_warms_up_then_compiles_and_dispatches() {
        let bytecode: Vec<u16> = vec![0x1052, 0x0003, 0x000f];
        let abi = mock_abi();
        let mut cache = JitCache::new();
        for _ in 0..(HOT_METHOD_THRESHOLD - 1) {
            assert!(cache.run_object(0, 500, &bytecode, 100, abi).is_none());
        }
        assert!(cache.run_object(0, 500, &bytecode, 100, abi).is_some());
    }

    #[test]
    fn object_decode_rejects_object_comparisons() {
        assert!(decode_object_subset(&[0x0038, 0x0002, 0x000f]).is_none());
        assert!(decode_object_subset(&[0x0132, 0x0002, 0x000f]).is_none());
        assert!(decode_object_subset(&[0x003a, 0x0002, 0x000f]).is_some());
    }

    #[test]
    fn run_object_sticky_rejects_unsupported() {
        let bytecode: Vec<u16> = vec![0x00f0, 0x000f];
        let abi = mock_abi();
        let mut cache = JitCache::new();
        for _ in 0..(HOT_METHOD_THRESHOLD + 3) {
            assert!(cache.run_object(0, 600, &bytecode, 100, abi).is_none());
        }
    }

    #[test]
    fn object_method_reads_a_field_and_computes() {
        let ops = vec![
            ObjectOp::IgetInt {
                dst: 0,
                obj: 1,
                field_index: 7,
                pc: 0,
            },
            ObjectOp::BinOp {
                dst: 0,
                a: 0,
                b: 0,
                kind: JitBinOp::Add,
            },
            ObjectOp::Const {
                dst: 2,
                literal: 100,
            },
            ObjectOp::BinOp {
                dst: 0,
                a: 0,
                b: 2,
                kind: JitBinOp::Add,
            },
            ObjectOp::Return { src: 0, pc: 0 },
        ];
        let abi = mock_abi();
        let compiled = compile_object_method(&ops, 100, abi).expect("should compile");
        let mut env = mock_env([0; 8], &[(7, 21)]);
        let env_ptr = &mut env as *mut MockEnv as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(env_ptr) }, 0);
        assert_eq!(env.returned, Some(142));
        assert_eq!(env.regs[0], 142);
    }

    #[test]
    fn compiled_code_calls_a_runtime_trampoline() {
        let compiled = compile_trampoline_call(add_ctx_counter).expect("should compile");
        let mut counter: i32 = 10;
        let ctx = &mut counter as *mut i32 as *mut std::ffi::c_void;
        assert_eq!(unsafe { compiled.call(ctx, 5) }, 16);
        assert_eq!(unsafe { compiled.call(ctx, 5) }, 17);
        assert_eq!(counter, 12);
    }

    #[test]
    fn decode_rejects_div_and_rem() {
        assert!(decode_integer_subset(&[0x0093, 0x0100, 0x000f]).is_none());
        assert!(decode_integer_subset(&[0x00b3, 0x000f]).is_none());
    }

    #[test]
    fn decodes_2addr_lit_and_unary_opcodes() {
        let bytecode: Vec<u16> = vec![0x10b0, 0x00d8, 0x0700, 0x007b, 0x000f];
        let ops = decode_integer_subset(&bytecode).expect("should decode");
        assert_eq!(
            ops,
            vec![
                JitOp::BinOp {
                    dst: 0,
                    a: 0,
                    b: 1,
                    kind: JitBinOp::Add,
                },
                JitOp::BinOpLit {
                    dst: 0,
                    src: 0,
                    literal: 7,
                    kind: JitLitOp::Add,
                },
                JitOp::UnOp {
                    dst: 0,
                    src: 0,
                    kind: JitUnOp::Neg,
                },
                JitOp::Return { src: 0 },
            ]
        );
        let compiled = compile(&ops, 2).expect("should compile");
        assert_eq!(compiled.call(&[3, 4]), -14);
    }
}

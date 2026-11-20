use crate::dex::DexTryBlock;
use crate::dex3oat::reg_type::RegType;

pub type RegisterLine = Vec<RegType>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnType {
    Void,
    Value(RegType),
}

pub trait VerifierTypes {
    fn field_type(&self, field_index: u32) -> Option<RegType>;
    fn method_return_type(&self, method_index: u32) -> Option<ReturnType>;
}

// cite: art reg_type.cc#Merge
// cite: art register_line.cc#MergeRegisters
fn merge_into(target: &mut RegisterLine, incoming: &RegisterLine) -> bool {
    let mut changed = false;
    for (slot, incoming_type) in target.iter_mut().zip(incoming.iter()) {
        let merged = slot.merge(*incoming_type);
        if merged != *slot {
            *slot = merged;
            changed = true;
        }
    }
    changed
}

fn read_defined(line: &RegisterLine, reg: usize) -> Option<RegType> {
    let ty = *line.get(reg)?;
    if ty == RegType::Undefined || ty == RegType::Conflict {
        return None;
    }
    Some(ty)
}

fn read_cat1_int(line: &RegisterLine, reg: usize) -> Option<()> {
    read_defined(line, reg)?.is_category1_int().then_some(())
}

fn read_ref(line: &RegisterLine, reg: usize) -> Option<()> {
    read_defined(line, reg)?.is_reference().then_some(())
}

fn read_float(line: &RegisterLine, reg: usize) -> Option<()> {
    read_defined(line, reg)?.is_float_compatible().then_some(())
}

fn read_cat1(line: &RegisterLine, reg: usize) -> Option<()> {
    read_defined(line, reg)?.is_float_compatible().then_some(())
}

fn read_wide(line: &RegisterLine, reg: usize) -> Option<()> {
    let lo = *line.get(reg)?;
    let hi = *line.get(reg + 1)?;
    (lo.is_wide_low() && hi.is_wide_high()).then_some(())
}

// cite: art register_line.cc#SetRegisterType
fn set1(line: &mut RegisterLine, reg: usize, ty: RegType) -> Option<()> {
    if reg > 0 && line[reg - 1].is_wide_low() {
        line[reg - 1] = RegType::Conflict;
    }
    if line.get(reg).copied()?.is_wide_low() {
        line[reg + 1] = RegType::Conflict;
    }
    *line.get_mut(reg)? = ty;
    Some(())
}

// cite: art register_line.cc#SetRegisterTypeWide
fn set_wide(line: &mut RegisterLine, reg: usize, lo: RegType, hi: RegType) -> Option<()> {
    if reg + 1 >= line.len() {
        return None;
    }
    if reg > 0 && line[reg - 1].is_wide_low() {
        line[reg - 1] = RegType::Conflict;
    }
    if reg + 2 < line.len() && line[reg + 1].is_wide_low() {
        line[reg + 2] = RegType::Conflict;
    }
    line[reg] = lo;
    line[reg + 1] = hi;
    Some(())
}

fn wide_pair(base: RegType) -> Option<(RegType, RegType)> {
    match base {
        RegType::LongLo => Some((RegType::LongLo, RegType::LongHi)),
        RegType::DoubleLo => Some((RegType::DoubleLo, RegType::DoubleHi)),
        _ => None,
    }
}

struct Step {
    successors: Vec<usize>,
}

// cite: art dex_instruction.h#kThrow
// cite: art method_verifier.cc#CodeFlowVerifyInstruction
fn can_throw(opcode: u8) -> bool {
    matches!(
        opcode,
        0x1a..=0x1c
            | 0x1d..=0x27
            | 0x44..=0x6d
            | 0x6e..=0x72
            | 0x74..=0x78
            | 0x93 | 0x94 | 0x9e | 0x9f
            | 0xb3 | 0xb4 | 0xbe | 0xbf
            | 0xd3 | 0xd4 | 0xdb | 0xdc
    )
}

// cite: art method_verifier.cc#CodeFlowVerifyMethod
// cite: art dex_file_structs.h#TryItem
fn handler_map(instructions_len: usize, try_blocks: &[DexTryBlock]) -> Vec<Vec<usize>> {
    let mut map = vec![Vec::new(); instructions_len];
    for block in try_blocks {
        let start = block.start_address as usize;
        let end = start + block.instruction_count as usize;
        let handlers: Vec<usize> = block
            .handlers
            .iter()
            .map(|handler| handler.address as usize)
            .collect();
        for slot in map.iter_mut().take(end.min(instructions_len)).skip(start) {
            slot.extend(handlers.iter().copied());
        }
    }
    map
}

pub fn verify_method(
    instructions: &[u16],
    registers_size: u16,
    parameter_types: &[RegType],
    try_blocks: &[DexTryBlock],
    resolver: &dyn VerifierTypes,
) -> Option<Vec<Option<RegisterLine>>> {
    if instructions.is_empty() || parameter_types.len() > registers_size as usize {
        return None;
    }
    let result_slot = registers_size as usize;
    let line_len = result_slot + 1;

    let mut entry = vec![RegType::Undefined; line_len];
    let first_param = result_slot - parameter_types.len();
    for (offset, ty) in parameter_types.iter().enumerate() {
        entry[first_param + offset] = *ty;
    }

    let handlers = handler_map(instructions.len(), try_blocks);
    let mut lines: Vec<Option<RegisterLine>> = vec![None; instructions.len()];
    lines[0] = Some(entry);
    let mut worklist = vec![0usize];

    while let Some(pc) = worklist.pop() {
        let entry_line = lines[pc].clone()?;
        let mut work = entry_line.clone();
        let step = step_instruction(instructions, pc, &mut work, result_slot, resolver)?;
        let mut edges: Vec<(usize, &RegisterLine)> =
            step.successors.iter().map(|s| (*s, &work)).collect();
        if can_throw((instructions[pc] & 0xff) as u8) {
            for &handler_pc in &handlers[pc] {
                edges.push((handler_pc, &entry_line));
            }
        }
        for (successor, incoming) in edges {
            if successor >= instructions.len() {
                return None;
            }
            match &mut lines[successor] {
                Some(existing) => {
                    if merge_into(existing, incoming) {
                        worklist.push(successor);
                    }
                }
                slot @ None => {
                    *slot = Some(incoming.clone());
                    worklist.push(successor);
                }
            }
        }
    }

    Some(lines)
}

fn code_unit(instructions: &[u16], index: usize) -> Option<u16> {
    instructions.get(index).copied()
}

fn branch_target(pc: usize, offset: i32) -> Option<usize> {
    let target = pc as i64 + offset as i64;
    if target < 0 {
        return None;
    }
    Some(target as usize)
}

// cite: art bytecode_utils.h#DexSwitchTable, GetEntryAt
fn switch_targets(instructions: &[u16], pc: usize) -> Option<Vec<usize>> {
    let low = code_unit(instructions, pc + 1)? as u32;
    let high = code_unit(instructions, pc + 2)? as u32;
    let offset = (low | (high << 16)) as i32;
    let payload = usize::try_from(pc as i64 + offset as i64).ok()?;
    let ident = code_unit(instructions, payload)?;
    let size = code_unit(instructions, payload + 1)? as usize;
    let targets_start = match ident {
        0x0100 => payload + 4,
        0x0200 => payload + 2 + size * 2,
        _ => return None,
    };
    let mut targets = Vec::with_capacity(size);
    for index in 0..size {
        let t_low = code_unit(instructions, targets_start + index * 2)? as u32;
        let t_high = code_unit(instructions, targets_start + index * 2 + 1)? as u32;
        let t_offset = (t_low | (t_high << 16)) as i32;
        targets.push(usize::try_from(pc as i64 + t_offset as i64).ok()?);
    }
    Some(targets)
}

// cite: art method_verifier.cc#CodeFlowVerifyMethod
// cite: art dex_instruction.h#SizeInCodeUnits, Format
fn step_instruction(
    instructions: &[u16],
    pc: usize,
    line: &mut RegisterLine,
    result_slot: usize,
    resolver: &dyn VerifierTypes,
) -> Option<Step> {
    let insn = code_unit(instructions, pc)?;
    let opcode = (insn & 0x00ff) as u8;
    let a4 = ((insn >> 8) & 0x0f) as usize;
    let b4 = ((insn >> 12) & 0x0f) as usize;
    let a8 = ((insn >> 8) & 0xff) as usize;

    let fallthrough = |width: usize| Step {
        successors: vec![pc + width],
    };

    match opcode {
        0x00 => Some(fallthrough(1)),
        0x01 => {
            let ty = read_defined(line, b4)?;
            if ty.is_wide_low() || ty.is_wide_high() {
                return None;
            }
            set1(line, a4, ty)?;
            Some(fallthrough(1))
        }
        0x07 => {
            read_ref(line, b4)?;
            let ty = read_defined(line, b4)?;
            set1(line, a4, ty)?;
            Some(fallthrough(1))
        }
        0x04 => {
            read_wide(line, b4)?;
            let lo = line[b4];
            let hi = line[b4 + 1];
            set_wide(line, a4, lo, hi)?;
            Some(fallthrough(1))
        }
        0x0a => {
            let ty = *line.get(result_slot)?;
            if ty.is_wide_low() || ty.is_wide_high() || ty == RegType::Undefined {
                return None;
            }
            set1(line, a8, ty)?;
            line[result_slot] = RegType::Undefined;
            Some(fallthrough(1))
        }
        0x0c => {
            let ty = *line.get(result_slot)?;
            if !ty.is_reference() {
                return None;
            }
            set1(line, a8, ty)?;
            line[result_slot] = RegType::Undefined;
            Some(fallthrough(1))
        }
        0x0b => {
            let lo = *line.get(result_slot)?;
            let (lo, hi) = wide_pair(lo)?;
            set_wide(line, a8, lo, hi)?;
            line[result_slot] = RegType::Undefined;
            Some(fallthrough(1))
        }
        0x0e => Some(Step {
            successors: Vec::new(),
        }),
        0x0f => {
            read_cat1(line, a8)?;
            Some(Step {
                successors: Vec::new(),
            })
        }
        0x11 => {
            read_ref(line, a8)?;
            Some(Step {
                successors: Vec::new(),
            })
        }
        0x10 => {
            read_wide(line, a8)?;
            Some(Step {
                successors: Vec::new(),
            })
        }
        0x12 => {
            let literal = ((insn as i16) >> 12) as i32;
            let ty = if literal == 0 {
                RegType::Zero
            } else {
                RegType::Const32
            };
            set1(line, a4, ty)?;
            Some(fallthrough(1))
        }
        0x13 => {
            let literal = code_unit(instructions, pc + 1)? as i16 as i32;
            let ty = if literal == 0 {
                RegType::Zero
            } else {
                RegType::Const32
            };
            set1(line, a8, ty)?;
            Some(fallthrough(2))
        }
        0x15 => {
            let literal = code_unit(instructions, pc + 1)? as i32;
            let ty = if literal == 0 {
                RegType::Zero
            } else {
                RegType::Const32
            };
            set1(line, a8, ty)?;
            Some(fallthrough(2))
        }
        0x14 => {
            let low = code_unit(instructions, pc + 1)? as i32;
            let high = code_unit(instructions, pc + 2)? as i32;
            let literal = low | (high << 16);
            let ty = if literal == 0 {
                RegType::Zero
            } else {
                RegType::Const32
            };
            set1(line, a8, ty)?;
            Some(fallthrough(3))
        }
        0x1a | 0x1c => {
            set1(line, a8, RegType::Reference)?;
            Some(fallthrough(2))
        }
        0x1b => {
            set1(line, a8, RegType::Reference)?;
            Some(fallthrough(3))
        }
        0x0d => {
            set1(line, a8, RegType::Reference)?;
            Some(fallthrough(1))
        }
        0x7b | 0x7c => {
            read_cat1_int(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(1))
        }
        0x21 => {
            read_ref(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(1))
        }
        0x23 => {
            read_cat1_int(line, b4)?;
            set1(line, a4, RegType::Reference)?;
            Some(fallthrough(2))
        }
        0x24 | 0x25 => {
            line[result_slot] = RegType::Reference;
            Some(fallthrough(3))
        }
        0x26 => {
            read_ref(line, a8)?;
            Some(fallthrough(3))
        }
        0x44..=0x4a => {
            let unit = code_unit(instructions, pc + 1)?;
            read_ref(line, (unit & 0xff) as usize)?;
            read_cat1_int(line, ((unit >> 8) & 0xff) as usize)?;
            match opcode {
                0x45 => set_wide(line, a8, RegType::LongLo, RegType::LongHi)?,
                0x46 => set1(line, a8, RegType::Reference)?,
                _ => set1(line, a8, RegType::Int32)?,
            }
            Some(fallthrough(2))
        }
        0x4b..=0x51 => {
            let unit = code_unit(instructions, pc + 1)?;
            read_ref(line, (unit & 0xff) as usize)?;
            read_cat1_int(line, ((unit >> 8) & 0xff) as usize)?;
            read_defined(line, a8)?;
            Some(fallthrough(2))
        }
        0x2b | 0x2c => {
            read_cat1_int(line, a8)?;
            let mut successors = switch_targets(instructions, pc)?;
            successors.push(pc + 3);
            Some(Step { successors })
        }
        0x22 => {
            set1(line, a8, RegType::Reference)?;
            Some(fallthrough(2))
        }
        0x28 => {
            let offset = ((insn >> 8) as u8) as i8 as i32;
            Some(Step {
                successors: vec![branch_target(pc, offset)?],
            })
        }
        0x29 => {
            let offset = code_unit(instructions, pc + 1)? as i16 as i32;
            Some(Step {
                successors: vec![branch_target(pc, offset)?],
            })
        }
        0x32..=0x37 => {
            if opcode <= 0x33 {
                read_defined(line, a4)?;
                read_defined(line, b4)?;
            } else {
                read_cat1_int(line, a4)?;
                read_cat1_int(line, b4)?;
            }
            let offset = code_unit(instructions, pc + 1)? as i16 as i32;
            Some(Step {
                successors: vec![pc + 2, branch_target(pc, offset)?],
            })
        }
        0x38..=0x3d => {
            if opcode <= 0x39 {
                read_defined(line, a8)?;
            } else {
                read_cat1_int(line, a8)?;
            }
            let offset = code_unit(instructions, pc + 1)? as i16 as i32;
            Some(Step {
                successors: vec![pc + 2, branch_target(pc, offset)?],
            })
        }
        0x52..=0x58 => {
            let field_index = code_unit(instructions, pc + 1)? as u32;
            read_ref(line, b4)?;
            let base = resolver.field_type(field_index)?;
            match wide_pair(base) {
                Some((lo, hi)) => set_wide(line, a4, lo, hi)?,
                None => set1(line, a4, base)?,
            }
            Some(fallthrough(2))
        }
        0x59..=0x5f => {
            let field_index = code_unit(instructions, pc + 1)? as u32;
            read_ref(line, b4)?;
            let base = resolver.field_type(field_index)?;
            match wide_pair(base) {
                Some(_) => read_wide(line, a4)?,
                None if base.is_reference() => read_ref(line, a4)?,
                None => read_cat1(line, a4)?,
            }
            Some(fallthrough(2))
        }
        0x60..=0x66 => {
            let field_index = code_unit(instructions, pc + 1)? as u32;
            let base = resolver.field_type(field_index)?;
            match wide_pair(base) {
                Some((lo, hi)) => set_wide(line, a8, lo, hi)?,
                None => set1(line, a8, base)?,
            }
            Some(fallthrough(2))
        }
        0x67..=0x6d => {
            let field_index = code_unit(instructions, pc + 1)? as u32;
            let base = resolver.field_type(field_index)?;
            match wide_pair(base) {
                Some(_) => read_wide(line, a8)?,
                None if base.is_reference() => read_ref(line, a8)?,
                None => read_cat1(line, a8)?,
            }
            Some(fallthrough(2))
        }
        0x6e..=0x72 => {
            let method_index = code_unit(instructions, pc + 1)? as u32;
            apply_invoke_result(line, result_slot, resolver, method_index)?;
            Some(fallthrough(3))
        }
        0x74..=0x78 => {
            let method_index = code_unit(instructions, pc + 1)? as u32;
            apply_invoke_result(line, result_slot, resolver, method_index)?;
            Some(fallthrough(3))
        }
        0x90..=0x9a => {
            let vb = (code_unit(instructions, pc + 1)? & 0xff) as usize;
            let vc = ((code_unit(instructions, pc + 1)? >> 8) & 0xff) as usize;
            read_cat1_int(line, vb)?;
            read_cat1_int(line, vc)?;
            set1(line, a8, RegType::Int32)?;
            Some(fallthrough(2))
        }
        0xb0..=0xba => {
            read_cat1_int(line, a4)?;
            read_cat1_int(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(1))
        }
        0xd0..=0xd7 => {
            read_cat1_int(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(2))
        }
        0xd8..=0xe2 => {
            let vb = (code_unit(instructions, pc + 1)? & 0xff) as usize;
            read_cat1_int(line, vb)?;
            set1(line, a8, RegType::Int32)?;
            Some(fallthrough(2))
        }
        0x02 => {
            let src = code_unit(instructions, pc + 1)? as usize;
            let ty = read_defined(line, src)?;
            if ty.is_wide_low() || ty.is_wide_high() {
                return None;
            }
            set1(line, a8, ty)?;
            Some(fallthrough(2))
        }
        0x03 => {
            let dst = code_unit(instructions, pc + 1)? as usize;
            let src = code_unit(instructions, pc + 2)? as usize;
            let ty = read_defined(line, src)?;
            if ty.is_wide_low() || ty.is_wide_high() {
                return None;
            }
            set1(line, dst, ty)?;
            Some(fallthrough(3))
        }
        0x05 => {
            let src = code_unit(instructions, pc + 1)? as usize;
            read_wide(line, src)?;
            let lo = line[src];
            let hi = line[src + 1];
            set_wide(line, a8, lo, hi)?;
            Some(fallthrough(2))
        }
        0x06 => {
            let dst = code_unit(instructions, pc + 1)? as usize;
            let src = code_unit(instructions, pc + 2)? as usize;
            read_wide(line, src)?;
            let lo = line[src];
            let hi = line[src + 1];
            set_wide(line, dst, lo, hi)?;
            Some(fallthrough(3))
        }
        0x08 => {
            let src = code_unit(instructions, pc + 1)? as usize;
            read_ref(line, src)?;
            let ty = read_defined(line, src)?;
            set1(line, a8, ty)?;
            Some(fallthrough(2))
        }
        0x09 => {
            let dst = code_unit(instructions, pc + 1)? as usize;
            let src = code_unit(instructions, pc + 2)? as usize;
            read_ref(line, src)?;
            let ty = read_defined(line, src)?;
            set1(line, dst, ty)?;
            Some(fallthrough(3))
        }
        0x1d | 0x1e => {
            read_ref(line, a8)?;
            Some(fallthrough(1))
        }
        0x1f => {
            read_ref(line, a8)?;
            set1(line, a8, RegType::Reference)?;
            Some(fallthrough(2))
        }
        0x20 => {
            read_ref(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(2))
        }
        0x27 => {
            read_ref(line, a8)?;
            Some(Step {
                successors: Vec::new(),
            })
        }
        0x16 | 0x19 => {
            set_wide(line, a8, RegType::Const64Lo, RegType::Const64Hi)?;
            Some(fallthrough(2))
        }
        0x17 => {
            set_wide(line, a8, RegType::Const64Lo, RegType::Const64Hi)?;
            Some(fallthrough(3))
        }
        0x18 => {
            set_wide(line, a8, RegType::Const64Lo, RegType::Const64Hi)?;
            Some(fallthrough(5))
        }
        0x9b..=0xa2 => {
            let unit = code_unit(instructions, pc + 1)?;
            read_wide(line, (unit & 0xff) as usize)?;
            read_wide(line, ((unit >> 8) & 0xff) as usize)?;
            set_wide(line, a8, RegType::LongLo, RegType::LongHi)?;
            Some(fallthrough(2))
        }
        0xa3..=0xa5 => {
            let unit = code_unit(instructions, pc + 1)?;
            read_wide(line, (unit & 0xff) as usize)?;
            read_cat1_int(line, ((unit >> 8) & 0xff) as usize)?;
            set_wide(line, a8, RegType::LongLo, RegType::LongHi)?;
            Some(fallthrough(2))
        }
        0xbb..=0xc2 => {
            read_wide(line, a4)?;
            read_wide(line, b4)?;
            set_wide(line, a4, RegType::LongLo, RegType::LongHi)?;
            Some(fallthrough(1))
        }
        0xc3..=0xc5 => {
            read_wide(line, a4)?;
            read_cat1_int(line, b4)?;
            set_wide(line, a4, RegType::LongLo, RegType::LongHi)?;
            Some(fallthrough(1))
        }
        0x7d | 0x7e => {
            read_wide(line, b4)?;
            set_wide(line, a4, RegType::LongLo, RegType::LongHi)?;
            Some(fallthrough(1))
        }
        0x31 => {
            let unit = code_unit(instructions, pc + 1)?;
            read_wide(line, (unit & 0xff) as usize)?;
            read_wide(line, ((unit >> 8) & 0xff) as usize)?;
            set1(line, a8, RegType::Int32)?;
            Some(fallthrough(2))
        }
        0x81 => {
            read_cat1_int(line, b4)?;
            set_wide(line, a4, RegType::LongLo, RegType::LongHi)?;
            Some(fallthrough(1))
        }
        0xa6..=0xaa => {
            let unit = code_unit(instructions, pc + 1)?;
            read_float(line, (unit & 0xff) as usize)?;
            read_float(line, ((unit >> 8) & 0xff) as usize)?;
            set1(line, a8, RegType::Float)?;
            Some(fallthrough(2))
        }
        0xab..=0xaf => {
            let unit = code_unit(instructions, pc + 1)?;
            read_wide(line, (unit & 0xff) as usize)?;
            read_wide(line, ((unit >> 8) & 0xff) as usize)?;
            set_wide(line, a8, RegType::DoubleLo, RegType::DoubleHi)?;
            Some(fallthrough(2))
        }
        0xc6..=0xca => {
            read_float(line, a4)?;
            read_float(line, b4)?;
            set1(line, a4, RegType::Float)?;
            Some(fallthrough(1))
        }
        0xcb..=0xcf => {
            read_wide(line, a4)?;
            read_wide(line, b4)?;
            set_wide(line, a4, RegType::DoubleLo, RegType::DoubleHi)?;
            Some(fallthrough(1))
        }
        0x7f => {
            read_float(line, b4)?;
            set1(line, a4, RegType::Float)?;
            Some(fallthrough(1))
        }
        0x80 => {
            read_wide(line, b4)?;
            set_wide(line, a4, RegType::DoubleLo, RegType::DoubleHi)?;
            Some(fallthrough(1))
        }
        0x2d | 0x2e => {
            let unit = code_unit(instructions, pc + 1)?;
            read_float(line, (unit & 0xff) as usize)?;
            read_float(line, ((unit >> 8) & 0xff) as usize)?;
            set1(line, a8, RegType::Int32)?;
            Some(fallthrough(2))
        }
        0x2f | 0x30 => {
            let unit = code_unit(instructions, pc + 1)?;
            read_wide(line, (unit & 0xff) as usize)?;
            read_wide(line, ((unit >> 8) & 0xff) as usize)?;
            set1(line, a8, RegType::Int32)?;
            Some(fallthrough(2))
        }
        0x82 => {
            read_cat1_int(line, b4)?;
            set1(line, a4, RegType::Float)?;
            Some(fallthrough(1))
        }
        0x83 => {
            read_cat1_int(line, b4)?;
            set_wide(line, a4, RegType::DoubleLo, RegType::DoubleHi)?;
            Some(fallthrough(1))
        }
        0x84 => {
            read_wide(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(1))
        }
        0x85 => {
            read_wide(line, b4)?;
            set1(line, a4, RegType::Float)?;
            Some(fallthrough(1))
        }
        0x86 => {
            read_wide(line, b4)?;
            set_wide(line, a4, RegType::DoubleLo, RegType::DoubleHi)?;
            Some(fallthrough(1))
        }
        0x87 => {
            read_float(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(1))
        }
        0x88 => {
            read_float(line, b4)?;
            set_wide(line, a4, RegType::LongLo, RegType::LongHi)?;
            Some(fallthrough(1))
        }
        0x89 => {
            read_float(line, b4)?;
            set_wide(line, a4, RegType::DoubleLo, RegType::DoubleHi)?;
            Some(fallthrough(1))
        }
        0x8a => {
            read_wide(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(1))
        }
        0x8b => {
            read_wide(line, b4)?;
            set_wide(line, a4, RegType::LongLo, RegType::LongHi)?;
            Some(fallthrough(1))
        }
        0x8c => {
            read_wide(line, b4)?;
            set1(line, a4, RegType::Float)?;
            Some(fallthrough(1))
        }
        0x8d | 0x8e | 0x8f => {
            read_cat1_int(line, b4)?;
            set1(line, a4, RegType::Int32)?;
            Some(fallthrough(1))
        }
        _ => None,
    }
}

fn apply_invoke_result(
    line: &mut RegisterLine,
    result_slot: usize,
    resolver: &dyn VerifierTypes,
    method_index: u32,
) -> Option<()> {
    match resolver.method_return_type(method_index)? {
        ReturnType::Void => line[result_slot] = RegType::Undefined,
        ReturnType::Value(base) => {
            line[result_slot] = match base {
                RegType::LongLo | RegType::DoubleLo => base,
                other => other,
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubTypes;

    impl VerifierTypes for StubTypes {
        fn field_type(&self, field_index: u32) -> Option<RegType> {
            match field_index {
                0 => Some(RegType::Int32),
                1 => Some(RegType::Reference),
                2 => Some(RegType::LongLo),
                _ => None,
            }
        }
        fn method_return_type(&self, method_index: u32) -> Option<ReturnType> {
            match method_index {
                0 => Some(ReturnType::Value(RegType::Int32)),
                1 => Some(ReturnType::Void),
                _ => None,
            }
        }
    }

    fn line_at(lines: &[Option<RegisterLine>], pc: usize) -> &RegisterLine {
        lines[pc].as_ref().expect("reachable")
    }

    // cite: art method_verifier.cc#CodeFlowVerifyMethod
    // cite: art register_line.h#SetRegisterType
    #[test]
    fn a_reused_register_has_distinct_types_per_pc() {
        let instructions = [0x5012u16, 0x000e];
        let lines = verify_method(&instructions, 1, &[RegType::Reference], &[], &StubTypes)
            .expect("verify");
        assert_eq!(line_at(&lines, 0)[0], RegType::Reference);
        assert_eq!(line_at(&lines, 1)[0], RegType::Const32);
    }

    // cite: art register_line.cc#MergeRegisters
    // cite: art reg_type.cc#Merge
    #[test]
    fn incompatible_types_merging_at_a_join_become_conflict() {
        let instructions = [
            0x0138u16, 0x0005, 0x1012, 0x0428, 0x0000, 0x001c, 0x0000, 0x000e,
        ];
        let lines =
            verify_method(&instructions, 2, &[RegType::Int32], &[], &StubTypes).expect("verify");
        assert_eq!(line_at(&lines, 7)[0], RegType::Conflict);
    }

    // cite: art method_verifier.cc#CodeFlowVerifyMethod
    #[test]
    fn a_back_edge_loop_reaches_a_fixpoint() {
        let instructions = [0x0012u16, 0x00b0, 0xff28];
        let lines = verify_method(&instructions, 1, &[], &[], &StubTypes).expect("verify");
        assert_eq!(line_at(&lines, 1)[0], RegType::Int32);
    }

    // cite: art method_verifier.cc#CodeFlowVerifyInstruction
    #[test]
    fn iget_types_its_destination_from_the_resolved_field() {
        let instructions = [0x1052u16, 0x0000, 0x000e];
        let lines = verify_method(&instructions, 2, &[RegType::Reference], &[], &StubTypes)
            .expect("verify");
        assert_eq!(line_at(&lines, 2)[0], RegType::Int32);
    }

    // cite: art register_line.h#GetRegisterType
    #[test]
    fn reading_an_uninitialized_register_fails_verification() {
        let instructions = [0x000fu16];
        assert!(verify_method(&instructions, 1, &[], &[], &StubTypes).is_none());
    }

    // cite: art method_verifier.cc#CodeFlowVerifyInstruction
    #[test]
    fn an_unmodeled_opcode_fails_verification() {
        let instructions = [0x003eu16];
        assert!(verify_method(&instructions, 1, &[RegType::Reference], &[], &StubTypes).is_none());
    }
}

use std::collections::HashMap;

use crate::dex3oat::reg_type::RegType;
use crate::dex3oat::wir::{
    ArrayElem, CmpBias, ConvKind, FloatBinKind, IfCmp, IntBinKind, LongBinKind, ShiftKind, WirOp,
};
use crate::jit::{
    CompiledMethod, JitBinOp, JitCmp, JitConvKind, JitFloatCmpBias, JitFloatOp, JitLitOp,
    JitLongBinOp, JitOp, JitShiftOp, JitUnOp, ObjectOp,
};

// cite: art dex_instruction_list.h#ADD_INT, SUB_INT, MUL_INT, AND_INT, OR_INT, XOR_INT, SHL_INT, SHR_INT, USHR_INT
fn bin_kind(kind: IntBinKind) -> Option<JitBinOp> {
    Some(match kind {
        IntBinKind::Add => JitBinOp::Add,
        IntBinKind::Sub => JitBinOp::Sub,
        IntBinKind::Mul => JitBinOp::Mul,
        IntBinKind::And => JitBinOp::And,
        IntBinKind::Or => JitBinOp::Or,
        IntBinKind::Xor => JitBinOp::Xor,
        IntBinKind::Shl => JitBinOp::Shl,
        IntBinKind::Shr => JitBinOp::Shr,
        IntBinKind::Ushr => JitBinOp::Ushr,
        IntBinKind::Rsub | IntBinKind::Div | IntBinKind::Rem => return None,
    })
}

// cite: art dex_instruction_list.h#ADD_INT_LIT8, RSUB_INT_LIT8, MUL_INT_LIT8, AND_INT_LIT8, OR_INT_LIT8, XOR_INT_LIT8, SHL_INT_LIT8, SHR_INT_LIT8, USHR_INT_LIT8
fn lit_kind(kind: IntBinKind) -> Option<JitLitOp> {
    Some(match kind {
        IntBinKind::Add => JitLitOp::Add,
        IntBinKind::Rsub => JitLitOp::RSub,
        IntBinKind::Mul => JitLitOp::Mul,
        IntBinKind::And => JitLitOp::And,
        IntBinKind::Or => JitLitOp::Or,
        IntBinKind::Xor => JitLitOp::Xor,
        IntBinKind::Shl => JitLitOp::Shl,
        IntBinKind::Shr => JitLitOp::Shr,
        IntBinKind::Ushr => JitLitOp::Ushr,
        IntBinKind::Sub | IntBinKind::Div | IntBinKind::Rem => return None,
    })
}

// cite: art dex_instruction_list.h#IF_EQ, IF_NE, IF_LT, IF_GE, IF_GT, IF_LE
fn cmp_kind(cmp: IfCmp) -> JitCmp {
    match cmp {
        IfCmp::Eq => JitCmp::Eq,
        IfCmp::Ne => JitCmp::Ne,
        IfCmp::Lt => JitCmp::Lt,
        IfCmp::Ge => JitCmp::Ge,
        IfCmp::Gt => JitCmp::Gt,
        IfCmp::Le => JitCmp::Le,
    }
}

// cite: art interpreter_switch_impl-inl.h#HandleCmpl, HandleCmpg
fn float_cmp_bias(bias: CmpBias) -> JitFloatCmpBias {
    match bias {
        CmpBias::Less => JitFloatCmpBias::Less,
        CmpBias::Greater => JitFloatCmpBias::Greater,
    }
}

// cite: art jit.cc#CompileMethod
pub fn wir_to_jitops(wir: &[(usize, WirOp)]) -> Option<Vec<JitOp>> {
    let index_of_pc: HashMap<usize, usize> = wir
        .iter()
        .enumerate()
        .map(|(index, (pc, _))| (*pc, index))
        .collect();
    let target = |pc: usize| index_of_pc.get(&pc).copied();

    let mut ops = Vec::with_capacity(wir.len());
    for (_, op) in wir {
        ops.push(match op {
            WirOp::ConstInt { dst, value } => JitOp::Const {
                dst: *dst,
                literal: *value,
            },
            WirOp::Move { dst, src, ty } if ty.is_category1_int() => JitOp::Move {
                dst: *dst,
                src: *src,
            },
            WirOp::IntBinOp { dst, a, b, kind } => JitOp::BinOp {
                dst: *dst,
                a: *a,
                b: *b,
                kind: bin_kind(*kind)?,
            },
            WirOp::IntBinLit {
                dst,
                src,
                literal,
                kind,
            } => JitOp::BinOpLit {
                dst: *dst,
                src: *src,
                literal: *literal,
                kind: lit_kind(*kind)?,
            },
            WirOp::IntNeg { dst, src } => JitOp::UnOp {
                dst: *dst,
                src: *src,
                kind: JitUnOp::Neg,
            },
            WirOp::IntNot { dst, src } => JitOp::UnOp {
                dst: *dst,
                src: *src,
                kind: JitUnOp::Not,
            },
            WirOp::Goto { target: pc } => JitOp::Goto {
                target: target(*pc)?,
            },
            WirOp::IfCmp {
                a,
                b,
                cmp,
                target: pc,
            } => JitOp::IfCmp {
                a: *a,
                b: *b,
                cmp: cmp_kind(*cmp),
                target: target(*pc)?,
            },
            WirOp::IfZero { a, cmp, target: pc } => JitOp::IfZero {
                a: *a,
                cmp: cmp_kind(*cmp),
                target: target(*pc)?,
            },
            WirOp::Return { src, ty } if ty.is_category1_int() => JitOp::Return { src: *src },
            _ => return None,
        });
    }
    Some(ops)
}

// cite: art jit.cc#CompileMethod
pub fn compile_wir(wir: &[(usize, WirOp)], register_count: u16) -> Option<CompiledMethod> {
    crate::jit::compile(&wir_to_jitops(wir)?, register_count)
}

// cite: art code_generator.cc#GenerateFrameEntry
fn is_int_field(ty: RegType) -> bool {
    ty.is_category1_int() || ty == RegType::Float
}

// cite: art optimizing_compiler.cc#TryCompile
pub fn wir_to_object_ops(
    wir: &[(usize, WirOp)],
    float_to_bits: &std::collections::HashSet<u32>,
    bits_to_float: &std::collections::HashSet<u32>,
) -> Option<Vec<ObjectOp>> {
    if wir
        .iter()
        .any(|(_, op)| matches!(op, WirOp::StaticGet { .. }))
    {
        return None;
    }
    let index_of_pc: HashMap<usize, usize> = wir
        .iter()
        .enumerate()
        .map(|(index, (pc, _))| (*pc, index))
        .collect();
    let target = |pc: usize| index_of_pc.get(&pc).copied();

    let mut ops = Vec::with_capacity(wir.len());
    let mut i = 0;
    while i < wir.len() {
        let (pc_usize, op) = &wir[i];
        let pc = u32::try_from(*pc_usize).ok()?;
        if let WirOp::Invoke {
            method_index, args, ..
        } = op
        {
            let is_f2b = float_to_bits.contains(method_index);
            let is_b2f = bits_to_float.contains(method_index);
            if (is_f2b || is_b2f) && args.len() == 1 {
                if let Some((_, WirOp::MoveResult { dst, .. })) = wir.get(i + 1) {
                    ops.push(if is_f2b {
                        ObjectOp::FloatBits {
                            dst: *dst,
                            src: args[0],
                            pc,
                        }
                    } else {
                        ObjectOp::BitsToFloat {
                            dst: *dst,
                            src: args[0],
                            pc,
                        }
                    });
                    i += 2;
                    continue;
                }
            }
        }
        ops.push(match op {
            WirOp::ConstInt { dst, value } => ObjectOp::Const {
                dst: *dst,
                literal: *value,
            },
            WirOp::IntBinOp { dst, a, b, kind } => ObjectOp::BinOp {
                dst: *dst,
                a: *a,
                b: *b,
                kind: bin_kind(*kind)?,
            },
            WirOp::Move { dst, src, ty } if is_int_field(*ty) || ty.is_reference() => {
                ObjectOp::Move {
                    dst: *dst,
                    src: *src,
                }
            }
            WirOp::FieldGet {
                dst,
                obj,
                ty,
                field_index,
                ..
            } => {
                if is_int_field(*ty) {
                    ObjectOp::IgetInt {
                        dst: *dst,
                        obj: *obj,
                        field_index: *field_index,
                        pc,
                    }
                } else if ty.is_reference() {
                    ObjectOp::IgetObject {
                        dst: *dst,
                        obj: *obj,
                        field_index: *field_index,
                        pc,
                    }
                } else {
                    return None;
                }
            }
            WirOp::FieldSet {
                obj,
                src,
                ty,
                field_index,
                ..
            } => {
                if is_int_field(*ty) || ty.is_reference() {
                    ObjectOp::IputField {
                        val: *src,
                        obj: *obj,
                        field_index: *field_index,
                        pc,
                    }
                } else {
                    return None;
                }
            }
            WirOp::IntBinLit {
                dst,
                src,
                literal,
                kind,
            } => ObjectOp::BinOpLit {
                dst: *dst,
                src: *src,
                literal: *literal,
                kind: lit_kind(*kind)?,
                pc,
            },
            WirOp::IntNeg { dst, src } => ObjectOp::UnOp {
                dst: *dst,
                src: *src,
                kind: JitUnOp::Neg,
                pc,
            },
            WirOp::IntNot { dst, src } => ObjectOp::UnOp {
                dst: *dst,
                src: *src,
                kind: JitUnOp::Not,
                pc,
            },
            WirOp::HandlerFieldGet {
                dst,
                obj,
                field_index,
                ty,
            } => {
                if is_int_field(*ty) {
                    ObjectOp::IgetInt {
                        dst: *dst,
                        obj: *obj,
                        field_index: *field_index,
                        pc,
                    }
                } else if ty.is_reference() {
                    ObjectOp::IgetObject {
                        dst: *dst,
                        obj: *obj,
                        field_index: *field_index,
                        pc,
                    }
                } else {
                    return None;
                }
            }
            WirOp::HandlerFieldSet {
                obj,
                src,
                field_index,
                ty,
            } => {
                if is_int_field(*ty) || ty.is_reference() {
                    ObjectOp::IputField {
                        val: *src,
                        obj: *obj,
                        field_index: *field_index,
                        pc,
                    }
                } else {
                    return None;
                }
            }
            WirOp::StaticGet {
                dst,
                field_index,
                ty,
            } => {
                if is_int_field(*ty) {
                    ObjectOp::SgetInt {
                        dst: *dst,
                        field_index: *field_index,
                        pc,
                    }
                } else if ty.is_reference() {
                    ObjectOp::SgetObject {
                        dst: *dst,
                        field_index: *field_index,
                        pc,
                    }
                } else {
                    return None;
                }
            }
            WirOp::NewInstance { dst, type_index } => ObjectOp::NewInstance {
                dst: *dst,
                type_index: *type_index,
                pc,
            },
            WirOp::ConstString { dst, string_index } => ObjectOp::ConstString {
                dst: *dst,
                string_index: *string_index,
                pc,
            },
            WirOp::ConstClass { dst, type_index } => ObjectOp::ConstClass {
                dst: *dst,
                type_index: *type_index,
                pc,
            },
            WirOp::InstanceOf {
                dst,
                reg,
                type_index,
            } => ObjectOp::InstanceOf {
                dst: *dst,
                reg: *reg,
                type_index: *type_index,
                pc,
            },
            WirOp::Invoke { .. } => ObjectOp::Invoke { pc },
            WirOp::MoveResult { dst, ty } => ObjectOp::MoveResult {
                dst: *dst,
                wide: matches!(ty, RegType::LongLo | RegType::DoubleLo),
                pc,
            },
            WirOp::Goto { target: t } => ObjectOp::Goto {
                target: target(*t)?,
            },
            WirOp::IfCmp {
                a,
                b,
                cmp,
                target: t,
            } => ObjectOp::IfCmp {
                a: *a,
                b: *b,
                cmp: cmp_kind(*cmp),
                target: target(*t)?,
                pc,
            },
            WirOp::IfZero { a, cmp, target: t } => ObjectOp::IfZero {
                a: *a,
                cmp: cmp_kind(*cmp),
                target: target(*t)?,
                pc,
            },
            WirOp::CheckCast { .. } => ObjectOp::Bail { pc },
            WirOp::Return { src, ty } => {
                if ty.is_reference() {
                    ObjectOp::ReturnObject { src: *src }
                } else if is_int_field(*ty) || matches!(ty, RegType::Float | RegType::LongLo) {
                    ObjectOp::Return { src: *src, pc }
                } else {
                    return None;
                }
            }
            WirOp::ReturnVoid => ObjectOp::ReturnVoid,
            // cite: art dex_instruction_list.h#THROW
            WirOp::Throw { .. } => ObjectOp::Bail { pc },
            WirOp::LongConst { dst, value } => ObjectOp::LongConst {
                dst: *dst,
                value: *value,
            },
            WirOp::LongBinOp { dst, a, b, kind } => {
                let kind = match kind {
                    LongBinKind::Add => JitLongBinOp::Add,
                    LongBinKind::Sub => JitLongBinOp::Sub,
                    LongBinKind::Mul => JitLongBinOp::Mul,
                    LongBinKind::And => JitLongBinOp::And,
                    LongBinKind::Or => JitLongBinOp::Or,
                    LongBinKind::Xor => JitLongBinOp::Xor,
                    LongBinKind::Div | LongBinKind::Rem => return None,
                };
                ObjectOp::LongBinOp {
                    dst: *dst,
                    a: *a,
                    b: *b,
                    kind,
                }
            }
            WirOp::FloatBinOp { dst, a, b, kind } => {
                let kind = match kind {
                    FloatBinKind::Add => JitFloatOp::Add,
                    FloatBinKind::Sub => JitFloatOp::Sub,
                    FloatBinKind::Mul => JitFloatOp::Mul,
                    FloatBinKind::Div => JitFloatOp::Div,
                    FloatBinKind::Rem => return None,
                };
                ObjectOp::FloatBinOp {
                    dst: *dst,
                    a: *a,
                    b: *b,
                    kind,
                    pc,
                }
            }
            // cite: art interpreter_switch_impl-inl.h#CMPL_FLOAT, CMPG_FLOAT
            WirOp::CmpFloat { dst, a, b, bias } => ObjectOp::CmpFloat {
                dst: *dst,
                a: *a,
                b: *b,
                bias: float_cmp_bias(*bias),
                pc,
            },
            WirOp::LongShift {
                dst,
                src,
                amount,
                kind,
            } => ObjectOp::LongShift {
                dst: *dst,
                src: *src,
                amount: *amount,
                kind: match kind {
                    ShiftKind::Shl => JitShiftOp::Shl,
                    ShiftKind::Shr => JitShiftOp::Shr,
                    ShiftKind::Ushr => JitShiftOp::Ushr,
                },
                pc,
            },
            WirOp::IntToLong { dst, src } => ObjectOp::IntToLong {
                dst: *dst,
                src: *src,
                pc,
            },
            WirOp::Convert {
                dst,
                src,
                kind: ConvKind::LongToInt,
            } => ObjectOp::LongToInt {
                dst: *dst,
                src: *src,
                pc,
            },
            WirOp::Convert { dst, src, kind } => {
                let kind = match kind {
                    ConvKind::IntToFloat => JitConvKind::IntToFloat,
                    ConvKind::FloatToInt => JitConvKind::FloatToInt,
                    ConvKind::IntToByte => JitConvKind::IntToByte,
                    ConvKind::IntToChar => JitConvKind::IntToChar,
                    ConvKind::IntToShort => JitConvKind::IntToShort,
                    _ => return None,
                };
                ObjectOp::Convert {
                    dst: *dst,
                    src: *src,
                    kind,
                    pc,
                }
            }
            WirOp::ArrayGet {
                dst,
                array,
                index,
                elem,
            } => match elem {
                ArrayElem::Wide => return None,
                _ => ObjectOp::ArrayGet {
                    dst: *dst,
                    array: *array,
                    index: *index,
                    pc,
                },
            },
            _ => return None,
        });
        i += 1;
    }
    Some(ops)
}

pub fn object_codegen_reject_reason(
    wir: &[(usize, WirOp)],
    float_to_bits: &std::collections::HashSet<u32>,
    bits_to_float: &std::collections::HashSet<u32>,
) -> Option<String> {
    let index_of_pc: HashMap<usize, usize> = wir
        .iter()
        .enumerate()
        .map(|(index, (pc, _))| (*pc, index))
        .collect();
    let target = |pc: usize| index_of_pc.get(&pc).copied();

    let mut i = 0;
    while i < wir.len() {
        let (pc, op) = &wir[i];
        if u32::try_from(*pc).is_err() {
            return Some(format!("pc-overflow pc={pc} op={op:?}"));
        }
        if let WirOp::Invoke {
            method_index, args, ..
        } = op
        {
            let bitcast =
                float_to_bits.contains(method_index) || bits_to_float.contains(method_index);
            if bitcast
                && args.len() == 1
                && matches!(wir.get(i + 1), Some((_, WirOp::MoveResult { .. })))
            {
                i += 2;
                continue;
            }
        }
        let reason = match op {
            WirOp::ConstInt { .. } | WirOp::IntNeg { .. } | WirOp::IntNot { .. } => None,
            WirOp::StaticGet { .. } => Some("static-get-requires-interpreter-clinit".to_owned()),
            WirOp::NewInstance { .. }
            | WirOp::ConstString { .. }
            | WirOp::ConstClass { .. }
            | WirOp::InstanceOf { .. }
            | WirOp::Invoke { .. }
            | WirOp::MoveResult { .. }
            | WirOp::Throw { .. }
            | WirOp::ReturnVoid
            | WirOp::LongConst { .. }
            | WirOp::LongShift { .. }
            | WirOp::IntToLong { .. } => None,
            WirOp::IntBinOp { kind, .. } => bin_kind(*kind)
                .is_none()
                .then(|| format!("unsupported-int-bin {kind:?}")),
            WirOp::Move { ty, .. } => (!(is_int_field(*ty) || ty.is_reference()))
                .then(|| format!("unsupported-move-type {ty:?}")),
            WirOp::FieldGet { ty, .. } => (!(is_int_field(*ty) || ty.is_reference()))
                .then(|| format!("unsupported-field-get-type {ty:?}")),
            WirOp::FieldSet { ty, .. } => (!(is_int_field(*ty) || ty.is_reference()))
                .then(|| format!("unsupported-field-set-type {ty:?}")),
            WirOp::IntBinLit { kind, .. } => lit_kind(*kind)
                .is_none()
                .then(|| format!("unsupported-int-lit {kind:?}")),
            WirOp::HandlerFieldGet { ty, .. } => (!(is_int_field(*ty) || ty.is_reference()))
                .then(|| format!("unsupported-handler-get-type {ty:?}")),
            WirOp::HandlerFieldSet { ty, .. } => (!(is_int_field(*ty) || ty.is_reference()))
                .then(|| format!("unsupported-handler-set-type {ty:?}")),
            WirOp::Goto { target: t } => target(*t)
                .is_none()
                .then(|| format!("missing-goto-target {t}")),
            WirOp::IfCmp { target: t, .. } => target(*t)
                .is_none()
                .then(|| format!("missing-if-target {t}")),
            WirOp::IfZero { target: t, .. } => target(*t)
                .is_none()
                .then(|| format!("missing-ifz-target {t}")),
            WirOp::CheckCast { .. } => None,
            WirOp::Return { ty, .. } => {
                (!(ty.is_reference() || is_int_field(*ty) || matches!(ty, RegType::LongLo)))
                    .then(|| format!("unsupported-return-type {ty:?}"))
            }
            WirOp::LongBinOp { kind, .. } => matches!(kind, LongBinKind::Div | LongBinKind::Rem)
                .then(|| format!("unsupported-long-bin {kind:?}")),
            WirOp::FloatBinOp { kind, .. } => {
                matches!(kind, FloatBinKind::Rem).then(|| format!("unsupported-float-bin {kind:?}"))
            }
            WirOp::CmpFloat { .. } => None,
            WirOp::Convert {
                kind: ConvKind::LongToInt,
                ..
            } => None,
            WirOp::Convert { kind, .. } => match kind {
                ConvKind::IntToFloat
                | ConvKind::FloatToInt
                | ConvKind::IntToByte
                | ConvKind::IntToChar
                | ConvKind::IntToShort => None,
                _ => Some(format!("unsupported-convert {kind:?}")),
            },
            WirOp::ArrayGet { elem, .. } => {
                matches!(elem, ArrayElem::Wide).then(|| "unsupported-wide-array-get".to_string())
            }
            other => Some(format!("unsupported-wir {other:?}")),
        };
        if let Some(reason) = reason {
            return Some(format!("pc={pc} {reason} op={op:?}"));
        }
        i += 1;
    }
    Some("unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex::DexCodeItem;
    use crate::dex3oat::reg_type::RegType;
    use crate::dex3oat::verifier::{verify_method, ReturnType, VerifierTypes};
    use crate::dex3oat::wir::{lower_method, CallKind, WirResolver};
    use crate::dex_interpreter::{
        interpret_code_with_initial_arguments_in_context, DexExecutionResult, DexInvocation,
        DexInvocationHandler, DexInvocationResult, DexValue,
    };
    use crate::error::{Result, WinedroidError};

    struct NoInvoke;
    impl DexInvocationHandler for NoInvoke {
        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            Err(WinedroidError::new("codegen test invoked nothing"))
        }
    }

    struct Empty;
    impl VerifierTypes for Empty {
        fn field_type(&self, _field_index: u32) -> Option<RegType> {
            None
        }
        fn method_return_type(&self, _method_index: u32) -> Option<ReturnType> {
            None
        }
    }
    impl WirResolver for Empty {
        fn field_slot_and_type(&self, _field_index: u32) -> Option<(u16, RegType)> {
            None
        }
        fn call_kind(&self, _method_index: u32) -> Option<CallKind> {
            None
        }
        fn array_component(&self, _type_index: u32) -> Option<RegType> {
            None
        }
        fn field_type(&self, _field_index: u32) -> Option<RegType> {
            None
        }
    }

    fn code_item(registers_size: u16, ins_size: u16, instructions: Vec<u16>) -> DexCodeItem {
        DexCodeItem {
            registers_size,
            ins_size,
            outs_size: 0,
            tries_size: 0,
            debug_info_offset: 0,
            instructions,
            try_blocks: Vec::new(),
        }
    }

    fn interpreter_int(instructions: &[u16], registers_size: u16, params: &[DexValue]) -> i32 {
        let code = code_item(registers_size, params.len() as u16, instructions.to_vec());
        match interpret_code_with_initial_arguments_in_context(
            &code,
            Some(params),
            &mut NoInvoke,
            0,
        ) {
            Ok(DexExecutionResult::Int(v)) => v,
            Ok(DexExecutionResult::Value(DexValue::Int(v))) => v,
            other => panic!("interpreter did not return an int: {other:?}"),
        }
    }

    fn native_int(instructions: &[u16], registers_size: u16, params: &[i32]) -> i32 {
        let types = vec![RegType::Int32; params.len()];
        let verified = verify_method(instructions, registers_size, &types, &[], &Empty).unwrap();
        let wir = lower_method(instructions, &verified, &Empty).unwrap();
        let compiled = compile_wir(&wir, registers_size).expect("int subset compiles");
        let mut registers = vec![0i32; registers_size as usize];
        let first_param = registers_size as usize - params.len();
        registers[first_param..].copy_from_slice(params);
        compiled.call(&registers)
    }

    // cite: art interpreter.cc#Execute
    #[test]
    fn a_compiled_add_matches_the_interpreter() {
        let instructions = [0x0090u16, 0x0302, 0x000f];
        for (a, b) in [(2, 3), (-5, 9), (i32::MAX, 1), (0, 0)] {
            let native = native_int(&instructions, 4, &[a, b]);
            let interp = interpreter_int(&instructions, 4, &[DexValue::Int(a), DexValue::Int(b)]);
            assert_eq!(native, interp, "add {a}+{b}");
        }
    }

    // cite: art interpreter_switch_impl-inl.h#IF_GE
    #[test]
    fn a_compiled_branch_matches_the_interpreter() {
        let instructions = [0x3235u16, 0x0003, 0x020f, 0x030f];
        for (a, b) in [(2, 3), (5, 1), (7, 7), (-4, -4), (i32::MIN, i32::MAX)] {
            let native = native_int(&instructions, 4, &[a, b]);
            let interp = interpreter_int(&instructions, 4, &[DexValue::Int(a), DexValue::Int(b)]);
            assert_eq!(native, interp, "min {a},{b}");
        }
    }

    // cite: art interpreter_switch_impl-inl.h#ADD_INT_LIT8
    #[test]
    fn a_compiled_literal_op_matches_the_interpreter() {
        let instructions = [0x00d8u16, 0x0701, 0x000f];
        for a in [0i32, 1, -1, 100, i32::MAX] {
            let native = native_int(&instructions, 2, &[a]);
            let interp = interpreter_int(&instructions, 2, &[DexValue::Int(a)]);
            assert_eq!(native, interp, "add-lit {a}");
        }
    }

    // cite: art dex_instruction_list.h#SHR_INT_LIT8, THROW
    #[test]
    fn object_codegen_lowers_lit8_shift_and_throw_bailout() {
        let empty = std::collections::HashSet::new();
        let ops = wir_to_object_ops(
            &[
                (
                    0,
                    WirOp::IntBinLit {
                        dst: 1,
                        src: 0,
                        literal: 33,
                        kind: IntBinKind::Shr,
                    },
                ),
                (1, WirOp::Throw { reg: 0 }),
            ],
            &empty,
            &empty,
        )
        .expect("object ops");
        assert_eq!(
            ops,
            vec![
                ObjectOp::BinOpLit {
                    dst: 1,
                    src: 0,
                    literal: 33,
                    kind: JitLitOp::Shr,
                    pc: 0,
                },
                ObjectOp::Bail { pc: 1 },
            ]
        );
    }

    // cite: art interpreter_switch_impl-inl.h#CMPL_FLOAT
    #[test]
    fn object_codegen_lowers_float_compare() {
        let empty = std::collections::HashSet::new();
        let ops = wir_to_object_ops(
            &[(
                4,
                WirOp::CmpFloat {
                    dst: 0,
                    a: 1,
                    b: 2,
                    bias: CmpBias::Less,
                },
            )],
            &empty,
            &empty,
        )
        .expect("object ops");

        assert_eq!(
            ops,
            vec![ObjectOp::CmpFloat {
                dst: 0,
                a: 1,
                b: 2,
                bias: JitFloatCmpBias::Less,
                pc: 4,
            }]
        );
    }

    // cite: art dex_instruction_list.h#CHECK_CAST
    #[test]
    fn object_codegen_lowers_check_cast_to_bailout() {
        let empty = std::collections::HashSet::new();
        let ops = wir_to_object_ops(
            &[(
                9,
                WirOp::CheckCast {
                    reg: 1,
                    type_index: 42,
                },
            )],
            &empty,
            &empty,
        )
        .expect("object ops");

        assert_eq!(ops, vec![ObjectOp::Bail { pc: 9 }]);
    }

    // cite: art nterp.cc#NterpGetStaticField
    #[test]
    fn object_codegen_rejects_static_get_that_can_run_class_initialization() {
        let empty = std::collections::HashSet::new();
        let wir = [(
            5,
            WirOp::StaticGet {
                dst: 0,
                field_index: 7,
                ty: RegType::Int32,
            },
        )];

        assert!(wir_to_object_ops(&wir, &empty, &empty).is_none());
        assert_eq!(
            object_codegen_reject_reason(&wir, &empty, &empty),
            Some(
                "pc=5 static-get-requires-interpreter-clinit op=StaticGet { dst: 0, field_index: 7, ty: Int32 }"
                    .to_owned()
            )
        );
    }
}

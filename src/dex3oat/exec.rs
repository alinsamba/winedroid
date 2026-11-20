use std::collections::HashMap;

use crate::dex3oat::heap::{Dex3oatHeap, HeapValue};
use crate::dex3oat::reg_type::RegType;
use crate::dex3oat::wir::{
    ArrayElem, CmpBias, ConvKind, FloatBinKind, IfCmp, IntBinKind, LongBinKind, ShiftKind, WirOp,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecValue {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Reference(u32),
    Uninitialized,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecOutcome {
    Return(ExecValue),
    ReturnVoid,
    Deopt,
    Error,
}

impl From<HeapValue> for ExecValue {
    fn from(value: HeapValue) -> Self {
        match value {
            HeapValue::Int(v) => ExecValue::Int(v),
            HeapValue::Long(v) => ExecValue::Long(v),
            HeapValue::Float(v) => ExecValue::Float(v),
            HeapValue::Double(v) => ExecValue::Double(v),
            HeapValue::Null => ExecValue::Reference(0),
            HeapValue::Reference(id) => ExecValue::Reference(id),
        }
    }
}

fn as_int(value: ExecValue) -> Option<i32> {
    match value {
        ExecValue::Int(v) => Some(v),
        _ => None,
    }
}

pub(crate) enum Step {
    Next,
    Jump(usize),
    Finish(ExecOutcome),
}

// cite: art interpreter_common.h#DoIntDivide
// cite: art interpreter_switch_impl-inl.h#SHL_INT, SHR_INT, USHR_INT, RSUB_INT
pub(crate) fn int_binop(kind: IntBinKind, x: i32, y: i32) -> Option<i32> {
    Some(match kind {
        IntBinKind::Add => x.wrapping_add(y),
        IntBinKind::Sub => x.wrapping_sub(y),
        IntBinKind::Rsub => y.wrapping_sub(x),
        IntBinKind::Mul => x.wrapping_mul(y),
        IntBinKind::And => x & y,
        IntBinKind::Or => x | y,
        IntBinKind::Xor => x ^ y,
        IntBinKind::Shl => x.wrapping_shl((y & 0x1f) as u32),
        IntBinKind::Shr => x.wrapping_shr((y & 0x1f) as u32),
        IntBinKind::Ushr => ((x as u32).wrapping_shr((y & 0x1f) as u32)) as i32,
        IntBinKind::Div => {
            if y == 0 {
                return None;
            }
            x.wrapping_div(y)
        }
        IntBinKind::Rem => {
            if y == 0 {
                return None;
            }
            x.wrapping_rem(y)
        }
    })
}

fn read_long(frame: &[ExecValue], reg: u16) -> Option<i64> {
    match frame.get(reg as usize)? {
        ExecValue::Long(v) => Some(*v),
        _ => None,
    }
}

fn set_long_reg(frame: &mut [ExecValue], reg: u16, value: i64) -> Option<()> {
    *frame.get_mut(reg as usize)? = ExecValue::Long(value);
    if let Some(high) = frame.get_mut(reg as usize + 1) {
        *high = ExecValue::Uninitialized;
    }
    Some(())
}

// cite: art interpreter_common.h#DoLongDivide
fn long_binop(kind: LongBinKind, x: i64, y: i64) -> Option<i64> {
    Some(match kind {
        LongBinKind::Add => x.wrapping_add(y),
        LongBinKind::Sub => x.wrapping_sub(y),
        LongBinKind::Mul => x.wrapping_mul(y),
        LongBinKind::And => x & y,
        LongBinKind::Or => x | y,
        LongBinKind::Xor => x ^ y,
        LongBinKind::Div => {
            if y == 0 {
                return None;
            }
            x.wrapping_div(y)
        }
        LongBinKind::Rem => {
            if y == 0 {
                return None;
            }
            x.wrapping_rem(y)
        }
    })
}

// cite: art interpreter_switch_impl-inl.h#SHL_LONG, SHR_LONG, USHR_LONG
fn long_shift(kind: ShiftKind, x: i64, amount: i32) -> i64 {
    let bits = (amount & 0x3f) as u32;
    match kind {
        ShiftKind::Shl => x.wrapping_shl(bits),
        ShiftKind::Shr => x.wrapping_shr(bits),
        ShiftKind::Ushr => ((x as u64).wrapping_shr(bits)) as i64,
    }
}

fn read_float_val(frame: &[ExecValue], reg: u16) -> Option<f32> {
    match frame.get(reg as usize)? {
        ExecValue::Float(v) => Some(*v),
        ExecValue::Int(v) => Some(f32::from_bits(*v as u32)),
        _ => None,
    }
}

fn read_double_val(frame: &[ExecValue], reg: u16) -> Option<f64> {
    match frame.get(reg as usize)? {
        ExecValue::Double(v) => Some(*v),
        ExecValue::Long(v) => Some(f64::from_bits(*v as u64)),
        _ => None,
    }
}

fn set_double_reg(frame: &mut [ExecValue], reg: u16, value: f64) -> Option<()> {
    *frame.get_mut(reg as usize)? = ExecValue::Double(value);
    if let Some(high) = frame.get_mut(reg as usize + 1) {
        *high = ExecValue::Uninitialized;
    }
    Some(())
}

// cite: art interpreter_switch_impl-inl.h#ADD_FLOAT, SUB_FLOAT, MUL_FLOAT, DIV_FLOAT, REM_FLOAT
fn float_binop(kind: FloatBinKind, x: f32, y: f32) -> f32 {
    match kind {
        FloatBinKind::Add => x + y,
        FloatBinKind::Sub => x - y,
        FloatBinKind::Mul => x * y,
        FloatBinKind::Div => x / y,
        FloatBinKind::Rem => x % y,
    }
}

fn double_binop(kind: FloatBinKind, x: f64, y: f64) -> f64 {
    match kind {
        FloatBinKind::Add => x + y,
        FloatBinKind::Sub => x - y,
        FloatBinKind::Mul => x * y,
        FloatBinKind::Div => x / y,
        FloatBinKind::Rem => x % y,
    }
}

// cite: art interpreter_switch_impl-inl.h#HandleCmpl, HandleCmpg
fn cmp_bias<T: PartialOrd>(bias: CmpBias, x: T, y: T) -> i32 {
    match bias {
        CmpBias::Less => {
            if x > y {
                1
            } else if x == y {
                0
            } else {
                -1
            }
        }
        CmpBias::Greater => {
            if x < y {
                -1
            } else if x == y {
                0
            } else {
                1
            }
        }
    }
}

// cite: art entrypoint_utils.h#art_float_to_integral
// cite: art interpreter_switch_impl-inl.h#INT_TO_FLOAT, LONG_TO_INT, FLOAT_TO_INT, DOUBLE_TO_LONG, INT_TO_BYTE, INT_TO_CHAR, INT_TO_SHORT
fn convert(frame: &mut [ExecValue], dst: u16, src: u16, kind: ConvKind) -> Option<()> {
    match kind {
        ConvKind::IntToFloat => {
            let v = as_int(*frame.get(src as usize)?)?;
            *frame.get_mut(dst as usize)? = ExecValue::Float(v as f32);
        }
        ConvKind::IntToDouble => {
            let v = as_int(*frame.get(src as usize)?)?;
            set_double_reg(frame, dst, v as f64)?;
        }
        ConvKind::LongToInt => {
            let v = read_long(frame, src)?;
            *frame.get_mut(dst as usize)? = ExecValue::Int(v as i32);
        }
        ConvKind::LongToFloat => {
            let v = read_long(frame, src)?;
            *frame.get_mut(dst as usize)? = ExecValue::Float(v as f32);
        }
        ConvKind::LongToDouble => {
            let v = read_long(frame, src)?;
            set_double_reg(frame, dst, v as f64)?;
        }
        ConvKind::FloatToInt => {
            let v = read_float_val(frame, src)?;
            *frame.get_mut(dst as usize)? = ExecValue::Int(v as i32);
        }
        ConvKind::FloatToLong => {
            let v = read_float_val(frame, src)?;
            set_long_reg(frame, dst, v as i64)?;
        }
        ConvKind::FloatToDouble => {
            let v = read_float_val(frame, src)?;
            set_double_reg(frame, dst, v as f64)?;
        }
        ConvKind::DoubleToInt => {
            let v = read_double_val(frame, src)?;
            *frame.get_mut(dst as usize)? = ExecValue::Int(v as i32);
        }
        ConvKind::DoubleToLong => {
            let v = read_double_val(frame, src)?;
            set_long_reg(frame, dst, v as i64)?;
        }
        ConvKind::DoubleToFloat => {
            let v = read_double_val(frame, src)?;
            *frame.get_mut(dst as usize)? = ExecValue::Float(v as f32);
        }
        ConvKind::IntToByte => {
            let v = as_int(*frame.get(src as usize)?)?;
            *frame.get_mut(dst as usize)? = ExecValue::Int(v as i8 as i32);
        }
        ConvKind::IntToChar => {
            let v = as_int(*frame.get(src as usize)?)?;
            *frame.get_mut(dst as usize)? = ExecValue::Int(v as u16 as i32);
        }
        ConvKind::IntToShort => {
            let v = as_int(*frame.get(src as usize)?)?;
            *frame.get_mut(dst as usize)? = ExecValue::Int(v as i16 as i32);
        }
    }
    Some(())
}

// cite: art interpreter_switch_impl-inl.h#IF_EQ, IF_EQZ
pub(crate) fn compare_taken(cmp: IfCmp, a: ExecValue, b: ExecValue) -> Option<bool> {
    match (a, b) {
        (ExecValue::Int(x), ExecValue::Int(y)) => Some(match cmp {
            IfCmp::Eq => x == y,
            IfCmp::Ne => x != y,
            IfCmp::Lt => x < y,
            IfCmp::Ge => x >= y,
            IfCmp::Gt => x > y,
            IfCmp::Le => x <= y,
        }),
        (ExecValue::Reference(x), ExecValue::Reference(y)) => match cmp {
            IfCmp::Eq => Some(x == y),
            IfCmp::Ne => Some(x != y),
            _ => None,
        },
        _ => None,
    }
}

fn read_array_id(frame: &[ExecValue], reg: u16) -> Option<u32> {
    match frame.get(reg as usize)? {
        ExecValue::Reference(id) if *id != crate::dex3oat::heap::NULL_REFERENCE => Some(*id),
        _ => None,
    }
}

fn into_long(value: ExecValue) -> Option<i64> {
    match value {
        ExecValue::Long(v) => Some(v),
        _ => None,
    }
}

// cite: art array-inl.h#GetWithoutChecks
fn array_get(
    frame: &[ExecValue],
    heap: &Dex3oatHeap,
    array: u16,
    index: u16,
    elem: ArrayElem,
) -> Option<ExecValue> {
    let id = read_array_id(frame, array)?;
    let position = as_int(*frame.get(index as usize)?)?;
    let word = heap.array_get_word(id, position)?;
    Some(match elem {
        ArrayElem::Wide => ExecValue::Long(word as i64),
        ArrayElem::Object => ExecValue::Reference(word as u32),
        _ => ExecValue::Int(word as u32 as i32),
    })
}

// cite: art array-inl.h#SetWithoutChecks
fn array_set(
    frame: &mut [ExecValue],
    heap: &mut Dex3oatHeap,
    array: u16,
    index: u16,
    src: u16,
    elem: ArrayElem,
) -> Option<()> {
    let id = read_array_id(frame, array)?;
    let position = as_int(*frame.get(index as usize)?)?;
    let word = match elem {
        ArrayElem::Wide => read_long(frame, src)? as u64,
        ArrayElem::Object => match frame.get(src as usize)? {
            ExecValue::Reference(referent) => u64::from(*referent),
            _ => return None,
        },
        ArrayElem::Boolean => u64::from((as_int(*frame.get(src as usize)?)? & 1) as u32),
        ArrayElem::Byte => u64::from(as_int(*frame.get(src as usize)?)? as i8 as i32 as u32),
        ArrayElem::Char => u64::from(as_int(*frame.get(src as usize)?)? as u16 as u32),
        ArrayElem::Short => u64::from(as_int(*frame.get(src as usize)?)? as i16 as i32 as u32),
        ArrayElem::Int => u64::from(as_int(*frame.get(src as usize)?)? as u32),
    };
    heap.array_set_word(id, position, word)
}

fn store_field(
    heap: &mut Dex3oatHeap,
    id: u32,
    slot: u16,
    ty: RegType,
    value: ExecValue,
) -> Option<()> {
    match (ty, value) {
        (RegType::Int32 | RegType::Zero, ExecValue::Int(v)) => heap.set_int(id, slot, v),
        (RegType::Float, ExecValue::Float(v)) => heap.set_float(id, slot, v),
        (RegType::LongLo, ExecValue::Long(v)) => heap.set_long(id, slot, v),
        (RegType::DoubleLo, ExecValue::Double(v)) => heap.set_double(id, slot, v),
        (RegType::Reference | RegType::Null, ExecValue::Reference(v)) => {
            heap.set_reference(id, slot, v)
        }
        _ => None,
    }
}

pub(crate) fn step_op(
    op: &WirOp,
    frame: &mut [ExecValue],
    heap: &mut Dex3oatHeap,
    index_of_pc: &HashMap<usize, usize>,
) -> Option<Step> {
    Some(match op {
        WirOp::ConstInt { dst, value } => {
            *frame.get_mut(*dst as usize)? = ExecValue::Int(*value);
            Step::Next
        }
        WirOp::IntBinOp { dst, a, b, kind } => {
            let x = as_int(*frame.get(*a as usize)?)?;
            let y = as_int(*frame.get(*b as usize)?)?;
            match int_binop(*kind, x, y) {
                Some(result) => {
                    *frame.get_mut(*dst as usize)? = ExecValue::Int(result);
                    Step::Next
                }
                None => Step::Finish(ExecOutcome::Deopt),
            }
        }
        WirOp::IntBinLit {
            dst,
            src,
            literal,
            kind,
        } => {
            let x = as_int(*frame.get(*src as usize)?)?;
            match int_binop(*kind, x, *literal) {
                Some(result) => {
                    *frame.get_mut(*dst as usize)? = ExecValue::Int(result);
                    Step::Next
                }
                None => Step::Finish(ExecOutcome::Deopt),
            }
        }
        WirOp::Move { dst, src, .. } => {
            let value = *frame.get(*src as usize)?;
            *frame.get_mut(*dst as usize)? = value;
            Step::Next
        }
        WirOp::FieldGet {
            dst, obj, slot, ty, ..
        } => {
            let ExecValue::Reference(id) = *frame.get(*obj as usize)? else {
                return Some(Step::Finish(ExecOutcome::Error));
            };
            if id == 0 {
                return Some(Step::Finish(ExecOutcome::Deopt));
            }
            match heap.materialize(id, *slot, *ty) {
                Some(value) => {
                    *frame.get_mut(*dst as usize)? = value.into();
                    Step::Next
                }
                None => Step::Finish(ExecOutcome::Deopt),
            }
        }
        WirOp::FieldSet {
            obj, slot, src, ty, ..
        } => {
            let ExecValue::Reference(id) = *frame.get(*obj as usize)? else {
                return Some(Step::Finish(ExecOutcome::Error));
            };
            if id == 0 {
                return Some(Step::Finish(ExecOutcome::Deopt));
            }
            let value = *frame.get(*src as usize)?;
            match store_field(heap, id, *slot, *ty, value) {
                Some(()) => Step::Next,
                None => Step::Finish(ExecOutcome::Deopt),
            }
        }
        WirOp::LongConst { dst, value } => {
            set_long_reg(frame, *dst, *value)?;
            Step::Next
        }
        WirOp::LongBinOp { dst, a, b, kind } => {
            let x = read_long(frame, *a)?;
            let y = read_long(frame, *b)?;
            match long_binop(*kind, x, y) {
                Some(result) => {
                    set_long_reg(frame, *dst, result)?;
                    Step::Next
                }
                None => Step::Finish(ExecOutcome::Deopt),
            }
        }
        WirOp::LongShift {
            dst,
            src,
            amount,
            kind,
        } => {
            let x = read_long(frame, *src)?;
            let bits = as_int(*frame.get(*amount as usize)?)?;
            set_long_reg(frame, *dst, long_shift(*kind, x, bits))?;
            Step::Next
        }
        WirOp::LongNeg { dst, src } => {
            let x = read_long(frame, *src)?;
            set_long_reg(frame, *dst, x.wrapping_neg())?;
            Step::Next
        }
        WirOp::LongNot { dst, src } => {
            let x = read_long(frame, *src)?;
            set_long_reg(frame, *dst, !x)?;
            Step::Next
        }
        WirOp::CmpLong { dst, a, b } => {
            let x = read_long(frame, *a)?;
            let y = read_long(frame, *b)?;
            let result = (x > y) as i32 - (x < y) as i32;
            *frame.get_mut(*dst as usize)? = ExecValue::Int(result);
            Step::Next
        }
        WirOp::IntToLong { dst, src } => {
            let x = as_int(*frame.get(*src as usize)?)?;
            set_long_reg(frame, *dst, x as i64)?;
            Step::Next
        }
        WirOp::FloatBinOp { dst, a, b, kind } => {
            let x = read_float_val(frame, *a)?;
            let y = read_float_val(frame, *b)?;
            *frame.get_mut(*dst as usize)? = ExecValue::Float(float_binop(*kind, x, y));
            Step::Next
        }
        WirOp::DoubleBinOp { dst, a, b, kind } => {
            let x = read_double_val(frame, *a)?;
            let y = read_double_val(frame, *b)?;
            set_double_reg(frame, *dst, double_binop(*kind, x, y))?;
            Step::Next
        }
        WirOp::FloatNeg { dst, src } => {
            let x = read_float_val(frame, *src)?;
            *frame.get_mut(*dst as usize)? = ExecValue::Float(-x);
            Step::Next
        }
        WirOp::DoubleNeg { dst, src } => {
            let x = read_double_val(frame, *src)?;
            set_double_reg(frame, *dst, -x)?;
            Step::Next
        }
        WirOp::CmpFloat { dst, a, b, bias } => {
            let x = read_float_val(frame, *a)?;
            let y = read_float_val(frame, *b)?;
            *frame.get_mut(*dst as usize)? = ExecValue::Int(cmp_bias(*bias, x, y));
            Step::Next
        }
        WirOp::CmpDouble { dst, a, b, bias } => {
            let x = read_double_val(frame, *a)?;
            let y = read_double_val(frame, *b)?;
            *frame.get_mut(*dst as usize)? = ExecValue::Int(cmp_bias(*bias, x, y));
            Step::Next
        }
        WirOp::Convert { dst, src, kind } => {
            convert(frame, *dst, *src, *kind)?;
            Step::Next
        }
        WirOp::IntNeg { dst, src } => {
            let x = as_int(*frame.get(*src as usize)?)?;
            *frame.get_mut(*dst as usize)? = ExecValue::Int(x.wrapping_neg());
            Step::Next
        }
        WirOp::IntNot { dst, src } => {
            let x = as_int(*frame.get(*src as usize)?)?;
            *frame.get_mut(*dst as usize)? = ExecValue::Int(!x);
            Step::Next
        }
        // cite: art interpreter_switch_impl-inl.h#NEW_ARRAY
        // cite: art array.cc#Alloc
        WirOp::NewArray {
            dst,
            size,
            component,
        } => {
            let length = as_int(*frame.get(*size as usize)?)?;
            match heap.allocate_array(*component, length) {
                Some(id) => {
                    *frame.get_mut(*dst as usize)? = ExecValue::Reference(id);
                    Step::Next
                }
                None => Step::Finish(ExecOutcome::Deopt),
            }
        }
        // cite: art interpreter_switch_impl-inl.h#ARRAY_LENGTH
        WirOp::ArrayLength { dst, array } => match read_array_id(frame, *array) {
            Some(id) => match heap.array_length(id) {
                Some(length) => {
                    *frame.get_mut(*dst as usize)? = ExecValue::Int(length);
                    Step::Next
                }
                None => Step::Finish(ExecOutcome::Deopt),
            },
            None => Step::Finish(ExecOutcome::Deopt),
        },
        // cite: art interpreter_switch_impl-inl.h#AGET
        // cite: art array-inl.h#GetWithoutChecks
        WirOp::ArrayGet {
            dst,
            array,
            index,
            elem,
        } => match array_get(frame, heap, *array, *index, *elem) {
            Some(value) => {
                match elem {
                    ArrayElem::Wide => set_long_reg(frame, *dst, into_long(value)?)?,
                    _ => *frame.get_mut(*dst as usize)? = value,
                }
                Step::Next
            }
            None => Step::Finish(ExecOutcome::Deopt),
        },
        // cite: art interpreter_switch_impl-inl.h#APUT
        // cite: art array-inl.h#SetWithoutChecks
        WirOp::ArraySet {
            array,
            index,
            src,
            elem,
        } => match array_set(frame, heap, *array, *index, *src, *elem) {
            Some(()) => Step::Next,
            None => Step::Finish(ExecOutcome::Deopt),
        },
        WirOp::Invoke { .. }
        | WirOp::MoveResult { .. }
        | WirOp::CheckCast { .. }
        | WirOp::InstanceOf { .. }
        | WirOp::Throw { .. }
        | WirOp::ConstString { .. }
        | WirOp::ConstClass { .. }
        | WirOp::NewInstance { .. }
        | WirOp::StaticGet { .. }
        | WirOp::StaticPut { .. }
        | WirOp::HandlerFieldGet { .. }
        | WirOp::HandlerFieldSet { .. }
        | WirOp::MoveException { .. }
        | WirOp::FilledNewArray { .. }
        | WirOp::FillArrayData { .. }
        | WirOp::Switch { .. } => Step::Finish(ExecOutcome::Deopt),
        WirOp::Return { src, .. } => Step::Finish(ExecOutcome::Return(*frame.get(*src as usize)?)),
        WirOp::ReturnVoid => Step::Finish(ExecOutcome::ReturnVoid),
        WirOp::Goto { target } => Step::Jump(*index_of_pc.get(target)?),
        WirOp::IfCmp { a, b, cmp, target } => {
            let taken = compare_taken(*cmp, *frame.get(*a as usize)?, *frame.get(*b as usize)?)?;
            if taken {
                Step::Jump(*index_of_pc.get(target)?)
            } else {
                Step::Next
            }
        }
        WirOp::IfZero { a, cmp, target } => {
            let operand = *frame.get(*a as usize)?;
            let zero = match operand {
                ExecValue::Reference(_) => ExecValue::Reference(0),
                _ => ExecValue::Int(0),
            };
            let taken = compare_taken(*cmp, operand, zero)?;
            if taken {
                Step::Jump(*index_of_pc.get(target)?)
            } else {
                Step::Next
            }
        }
    })
}

pub fn execute(
    wir: &[(usize, WirOp)],
    frame: &mut [ExecValue],
    heap: &mut Dex3oatHeap,
) -> ExecOutcome {
    let index_of_pc: HashMap<usize, usize> = wir
        .iter()
        .enumerate()
        .map(|(index, (pc, _))| (*pc, index))
        .collect();

    let mut index = 0;
    while let Some((_, op)) = wir.get(index) {
        match step_op(op, frame, heap, &index_of_pc) {
            Some(Step::Next) => index += 1,
            Some(Step::Jump(target)) => index = target,
            Some(Step::Finish(outcome)) => return outcome,
            None => return ExecOutcome::Error,
        }
    }
    ExecOutcome::Error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex::{DexCodeItem, DexProto};
    use crate::dex3oat::verifier::{verify_method, ReturnType, VerifierTypes};
    use crate::dex3oat::wir::{lower_method, CallKind, WirResolver};
    use crate::dex_interpreter::{
        interpret_code_with_initial_arguments_in_context, DexArrayLength, DexArrayRead,
        DexArrayWrite, DexExecutionResult, DexInvocation, DexInvocationHandler,
        DexInvocationResult, DexNewArray, DexObjectRef, DexValue,
    };
    use crate::error::{Result, WinedroidError};

    struct NoInvoke;

    impl DexInvocationHandler for NoInvoke {
        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            Err(WinedroidError::new(
                "differential test method invoked nothing",
            ))
        }
    }

    #[derive(Default)]
    struct IntArrayHeap {
        arrays: Vec<Vec<i32>>,
    }

    fn object_id(value: &DexValue) -> Result<usize> {
        match value {
            DexValue::Object(DexObjectRef { id, .. }) => Ok(*id as usize),
            other => Err(WinedroidError::new(format!("not an array ref: {other:?}"))),
        }
    }

    impl DexInvocationHandler for IntArrayHeap {
        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            Err(WinedroidError::new("int-array test invoked nothing"))
        }
        fn allocate_array(&mut self, array: DexNewArray) -> Result<DexValue> {
            let id = self.arrays.len();
            self.arrays.push(vec![0i32; array.size.max(0) as usize]);
            Ok(DexValue::Object(DexObjectRef {
                id: id as u64,
                class_descriptor: std::rc::Rc::from("[I"),
            }))
        }
        fn array_length(&mut self, length: DexArrayLength) -> Result<i32> {
            Ok(self.arrays[object_id(&length.array)?].len() as i32)
        }
        fn read_array(&mut self, read: DexArrayRead) -> Result<DexValue> {
            Ok(DexValue::Int(
                self.arrays[object_id(&read.array)?][read.index as usize],
            ))
        }
        fn write_array(&mut self, write: DexArrayWrite) -> Result<()> {
            let DexValue::Int(value) = write.value else {
                return Err(WinedroidError::new("int-array write of a non-int"));
            };
            self.arrays[object_id(&write.array)?][write.index as usize] = value;
            Ok(())
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
            Some(RegType::Int32)
        }
        fn field_type(&self, _field_index: u32) -> Option<RegType> {
            Some(RegType::Int32)
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

    fn exec_matches_result(exec: ExecValue, result: &DexExecutionResult) -> bool {
        match (exec, result) {
            (ExecValue::Int(a), DexExecutionResult::Int(b)) => a == *b,
            (ExecValue::Int(a), DexExecutionResult::Value(DexValue::Int(b))) => a == *b,
            (ExecValue::Long(a), DexExecutionResult::Value(DexValue::Long(b))) => a == *b,
            (ExecValue::Float(a), DexExecutionResult::Value(DexValue::Float(b))) => {
                a.to_bits() == b.to_bits()
            }
            (ExecValue::Double(a), DexExecutionResult::Value(DexValue::Long(b))) => {
                a.to_bits() as i64 == *b
            }
            _ => false,
        }
    }

    fn assert_matches_interpreter(
        registers_size: u16,
        params: &[DexValue],
        param_types: &[RegType],
        instructions: &[u16],
    ) {
        let code = code_item(registers_size, params.len() as u16, instructions.to_vec());
        let interpreted =
            interpret_code_with_initial_arguments_in_context(&code, Some(params), &mut NoInvoke, 0);

        let verified = verify_method(instructions, registers_size, param_types, &[], &Empty)
            .expect("differential test method must verify");
        let wir = lower_method(instructions, &verified, &Empty)
            .expect("differential test method must lower");

        let mut frame = vec![ExecValue::Uninitialized; registers_size as usize];
        let first_param = registers_size as usize - params.len();
        for (offset, value) in params.iter().enumerate() {
            frame[first_param + offset] = match value {
                DexValue::Int(v) => ExecValue::Int(*v),
                DexValue::Long(v) => ExecValue::Long(*v),
                DexValue::Float(v) => ExecValue::Float(*v),
                other => panic!("unsupported differential param {other:?}"),
            };
        }
        let mut heap = Dex3oatHeap::new();
        let outcome = execute(&wir, &mut frame, &mut heap);

        match outcome {
            ExecOutcome::Return(value) => {
                let interpreted = interpreted.expect("interpreter must not fault here");
                assert!(
                    exec_matches_result(value, &interpreted),
                    "dex3oat {value:?} != interpreter {interpreted:?} for {instructions:x?}"
                );
            }
            ExecOutcome::ReturnVoid => {
                assert_eq!(interpreted.unwrap(), DexExecutionResult::Void);
            }
            ExecOutcome::Deopt => {}
            ExecOutcome::Error => {
                panic!("dex3oat executor errored on verified WIR {instructions:x?}");
            }
        }
    }

    // cite: art interpreter_switch_impl-inl.h#NEW_ARRAY, APUT, AGET
    #[test]
    fn a_self_contained_int_array_method_matches_the_interpreter() {
        let instructions = [
            0x1112u16, 0x1023, 0x0000, 0x0212, 0x034b, 0x0200, 0x0044, 0x0200, 0x000f,
        ];
        for n in [0i32, 1, -7, i32::MAX, i32::MIN] {
            let code = code_item(4, 1, instructions.to_vec());
            let mut handler = IntArrayHeap::default();
            let interpreted = interpret_code_with_initial_arguments_in_context(
                &code,
                Some(&[DexValue::Int(n)]),
                &mut handler,
                0,
            )
            .expect("array oracle runs");

            let verified = verify_method(&instructions, 4, &[RegType::Int32], &[], &Empty).unwrap();
            let wir = lower_method(&instructions, &verified, &Empty).unwrap();
            let mut frame = vec![ExecValue::Uninitialized; 4];
            frame[3] = ExecValue::Int(n);
            let mut heap = Dex3oatHeap::new();
            match execute(&wir, &mut frame, &mut heap) {
                ExecOutcome::Return(value) => assert!(
                    exec_matches_result(value, &interpreted),
                    "array {value:?} != interpreter {interpreted:?} for n={n}"
                ),
                other => panic!("int array method did not return: {other:?} for n={n}"),
            }
        }
    }

    fn next_random(state: &mut u64) -> i32 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 33) as i32
    }

    fn binop_method(opcode: u16) -> Vec<u16> {
        vec![opcode, 0x0302, 0x000f]
    }

    // cite: art interpreter_switch_impl-inl.h#ADD_INT, SUB_INT, MUL_INT, DIV_INT, REM_INT, AND_INT, OR_INT, XOR_INT, SHL_INT, SHR_INT, USHR_INT
    #[test]
    fn int_binops_match_the_interpreter_over_random_inputs() {
        let opcodes: [u16; 11] = [
            0x0090, 0x0091, 0x0092, 0x0093, 0x0094, 0x0095, 0x0096, 0x0097, 0x0098, 0x0099, 0x009a,
        ];
        let edge = [
            0i32,
            1,
            -1,
            i32::MIN,
            i32::MAX,
            2,
            -2,
            31,
            32,
            33,
            0x7fff_ffff,
        ];
        let mut state = 0x1234_5678_9abc_def0u64;
        for opcode in opcodes {
            let method = binop_method(opcode);
            for &a in &edge {
                for &b in &edge {
                    assert_matches_interpreter(
                        4,
                        &[DexValue::Int(a), DexValue::Int(b)],
                        &[RegType::Int32, RegType::Int32],
                        &method,
                    );
                }
            }
            for _ in 0..500 {
                let a = next_random(&mut state);
                let b = next_random(&mut state);
                assert_matches_interpreter(
                    4,
                    &[DexValue::Int(a), DexValue::Int(b)],
                    &[RegType::Int32, RegType::Int32],
                    &method,
                );
            }
        }
    }

    // cite: art interpreter_switch_impl-inl.h#IF_LEZ, ADD_INT_2ADDR, SUB_INT_2ADDR, ADD_INT_LIT16, RSUB_INT, MUL_INT_LIT16, DIV_INT_LIT16, REM_INT_LIT16, AND_INT_LIT16, OR_INT_LIT16, XOR_INT_LIT16
    #[test]
    fn lit16_binops_match_the_interpreter() {
        let literals = [0i32, 1, -1, 2, -2, 100, -100, 32767, -32768];
        let mut state = 0x0bad_c0de_dead_beefu64;
        for opcode in 0xd0u16..=0xd7 {
            for &lit in &literals {
                let method = vec![0x2000 | opcode, lit as i16 as u16, 0x000f];
                for _ in 0..200 {
                    let src = next_random(&mut state);
                    assert_matches_interpreter(
                        3,
                        &[DexValue::Int(src)],
                        &[RegType::Int32],
                        &method,
                    );
                }
            }
        }
    }

    // cite: art interpreter_switch_impl-inl.h#ADD_INT_LIT8, RSUB_INT_LIT8, MUL_INT_LIT8, DIV_INT_LIT8, REM_INT_LIT8, AND_INT_LIT8, OR_INT_LIT8, XOR_INT_LIT8, SHL_INT_LIT8, SHR_INT_LIT8, USHR_INT_LIT8
    #[test]
    fn lit8_binops_match_the_interpreter() {
        let literals = [0i32, 1, -1, 2, -2, 31, 32, 127, -128];
        let mut state = 0xfeed_face_cafe_babeu64;
        for opcode in 0xd8u16..=0xe2 {
            for &lit in &literals {
                let method = vec![opcode, ((lit as u8 as u16) << 8) | 2, 0x000f];
                for _ in 0..200 {
                    let src = next_random(&mut state);
                    assert_matches_interpreter(
                        3,
                        &[DexValue::Int(src)],
                        &[RegType::Int32],
                        &method,
                    );
                }
            }
        }
    }

    #[test]
    fn a_counting_loop_matches_the_interpreter() {
        let method = vec![
            0x0012, 0x1112, 0x023d, 0x0005, 0x20b0, 0x12b1, 0xfc28, 0x000f,
        ];
        for n in [0i32, 1, 2, 5, 10, 100] {
            assert_matches_interpreter(3, &[DexValue::Int(n)], &[RegType::Int32], &method);
        }
    }

    // cite: art interpreter_common.h#DoIntDivide
    #[test]
    fn division_by_zero_deopts_rather_than_computing() {
        let method = binop_method(0x0093);
        let code = code_item(4, 2, method.clone());
        let verified =
            verify_method(&method, 4, &[RegType::Int32, RegType::Int32], &[], &Empty).unwrap();
        let wir = lower_method(&method, &verified, &Empty).unwrap();
        let mut frame = vec![
            ExecValue::Uninitialized,
            ExecValue::Uninitialized,
            ExecValue::Int(10),
            ExecValue::Int(0),
        ];
        let mut heap = Dex3oatHeap::new();
        assert_eq!(execute(&wir, &mut frame, &mut heap), ExecOutcome::Deopt);
        let _ = code;
    }

    #[test]
    fn field_get_reads_the_slot_and_set_writes_it() {
        use crate::dex3oat::wir::WirOp;
        let mut heap = Dex3oatHeap::new();
        let id = heap.allocate(0, 3);
        heap.set_int(id, 2, 42).unwrap();

        let getter = vec![
            (
                0,
                WirOp::FieldGet {
                    dst: 0,
                    obj: 1,
                    slot: 2,
                    ty: RegType::Int32,
                    field_index: 0,
                },
            ),
            (
                2,
                WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut frame = vec![ExecValue::Uninitialized, ExecValue::Reference(id)];
        assert_eq!(
            execute(&getter, &mut frame, &mut heap),
            ExecOutcome::Return(ExecValue::Int(42))
        );

        let setter = vec![
            (
                0,
                WirOp::FieldSet {
                    obj: 1,
                    slot: 2,
                    src: 0,
                    ty: RegType::Int32,
                    field_index: 0,
                },
            ),
            (2, WirOp::ReturnVoid),
        ];
        let mut frame = vec![ExecValue::Int(99), ExecValue::Reference(id)];
        assert_eq!(
            execute(&setter, &mut frame, &mut heap),
            ExecOutcome::ReturnVoid
        );
        assert_eq!(heap.get_int(id, 2), Some(99));
    }

    #[test]
    fn a_field_access_on_null_deopts() {
        use crate::dex3oat::wir::WirOp;
        let mut heap = Dex3oatHeap::new();
        let getter = vec![(
            0,
            WirOp::FieldGet {
                dst: 0,
                obj: 1,
                slot: 0,
                ty: RegType::Int32,
                field_index: 0,
            },
        )];
        let mut frame = vec![ExecValue::Uninitialized, ExecValue::Reference(0)];
        assert_eq!(execute(&getter, &mut frame, &mut heap), ExecOutcome::Deopt);
    }

    fn next_random_long(state: &mut u64) -> i64 {
        ((next_random(state) as i64) << 32) | (next_random(state) as u32 as i64)
    }

    fn long_params(a: i64, b: i64) -> Vec<DexValue> {
        vec![
            DexValue::Long(a),
            DexValue::Long(a),
            DexValue::Long(b),
            DexValue::Long(b),
        ]
    }

    fn long_types() -> Vec<RegType> {
        vec![
            RegType::LongLo,
            RegType::LongHi,
            RegType::LongLo,
            RegType::LongHi,
        ]
    }

    // cite: art interpreter_switch_impl-inl.h#ADD_LONG, SUB_LONG, MUL_LONG, DIV_LONG, REM_LONG, AND_LONG, OR_LONG, XOR_LONG
    #[test]
    fn long_binops_match_the_interpreter() {
        let mut state = 0x51ce_51ce_51ce_51ceu64;
        let edge = [0i64, 1, -1, i64::MIN, i64::MAX, i64::MIN + 1];
        for opcode in 0x9bu16..=0xa2 {
            let method = vec![opcode, 0x0604, 0x0010];
            for &a in &edge {
                for &b in &edge {
                    assert_matches_interpreter(8, &long_params(a, b), &long_types(), &method);
                }
            }
            for _ in 0..300 {
                let a = next_random_long(&mut state);
                let b = next_random_long(&mut state);
                assert_matches_interpreter(8, &long_params(a, b), &long_types(), &method);
            }
        }
    }

    // cite: art interpreter_switch_impl-inl.h#SHL_LONG, SHR_LONG, USHR_LONG
    #[test]
    fn long_shifts_match_the_interpreter() {
        let mut state = 0x5417_5417_5417_5417u64;
        for opcode in 0xa3u16..=0xa5 {
            let method = vec![opcode, 0x0402, 0x0010];
            for amount in [0i32, 1, 31, 32, 63, 64, 65, -1] {
                for _ in 0..100 {
                    let a = next_random_long(&mut state);
                    let params = vec![DexValue::Long(a), DexValue::Long(a), DexValue::Int(amount)];
                    let types = vec![RegType::LongLo, RegType::LongHi, RegType::Int32];
                    assert_matches_interpreter(5, &params, &types, &method);
                }
            }
        }
    }

    // cite: art interpreter_switch_impl-inl.h#CMP_LONG
    #[test]
    fn cmp_long_matches_the_interpreter() {
        let method = vec![0x0031, 0x0402, 0x000f];
        let mut state = 0xc319_c319_c319_c319u64;
        let edge = [0i64, 1, -1, i64::MIN, i64::MAX];
        for &a in &edge {
            for &b in &edge {
                assert_matches_interpreter(6, &long_params(a, b), &long_types(), &method);
            }
        }
        for _ in 0..500 {
            let a = next_random_long(&mut state);
            let b = next_random_long(&mut state);
            assert_matches_interpreter(6, &long_params(a, b), &long_types(), &method);
        }
    }

    // cite: art interpreter_switch_impl-inl.h#CONST_WIDE_16, CONST_WIDE, INT_TO_LONG
    #[test]
    fn const_wide_and_int_to_long_match_the_interpreter() {
        for v in [0i16, 1, -1, 12345, -12345, i16::MIN, i16::MAX] {
            let method = vec![0x0016, v as u16, 0x0010];
            assert_matches_interpreter(2, &[], &[], &method);
        }
        for v in [0i32, 1, -1, i32::MIN, i32::MAX, 999999] {
            let method = vec![0x2081, 0x0010];
            assert_matches_interpreter(3, &[DexValue::Int(v)], &[RegType::Int32], &method);
        }
    }

    fn float_vals() -> Vec<f32> {
        vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            3.14,
            -2.71,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            f32::MIN,
            1e30,
            -1e30,
        ]
    }

    fn double_params(a: f64, b: f64) -> Vec<DexValue> {
        vec![
            DexValue::Long(a.to_bits() as i64),
            DexValue::Long(a.to_bits() as i64),
            DexValue::Long(b.to_bits() as i64),
            DexValue::Long(b.to_bits() as i64),
        ]
    }

    fn double_types() -> Vec<RegType> {
        vec![
            RegType::DoubleLo,
            RegType::DoubleHi,
            RegType::DoubleLo,
            RegType::DoubleHi,
        ]
    }

    // cite: art interpreter_switch_impl-inl.h#ADD_FLOAT, SUB_FLOAT, MUL_FLOAT, DIV_FLOAT, REM_FLOAT
    #[test]
    fn float_binops_match_the_interpreter() {
        for opcode in 0x00a6u16..=0x00aa {
            let method = vec![opcode, 0x0302, 0x000f];
            for &a in &float_vals() {
                for &b in &float_vals() {
                    assert_matches_interpreter(
                        4,
                        &[DexValue::Float(a), DexValue::Float(b)],
                        &[RegType::Float, RegType::Float],
                        &method,
                    );
                }
            }
        }
    }

    // cite: art interpreter_switch_impl-inl.h#ADD_DOUBLE, SUB_DOUBLE, MUL_DOUBLE, DIV_DOUBLE, REM_DOUBLE
    #[test]
    fn double_binops_match_the_interpreter() {
        for opcode in 0x00abu16..=0x00af {
            let method = vec![opcode, 0x0402, 0x0010];
            for &a in &float_vals() {
                for &b in &float_vals() {
                    assert_matches_interpreter(
                        6,
                        &double_params(a as f64, b as f64),
                        &double_types(),
                        &method,
                    );
                }
            }
        }
    }

    // cite: art interpreter_switch_impl-inl.h#CMPL_FLOAT, CMPG_FLOAT, CMPL_DOUBLE, CMPG_DOUBLE
    #[test]
    fn float_and_double_compares_match_the_interpreter() {
        for opcode in [0x002du16, 0x002e] {
            let method = vec![opcode, 0x0302, 0x000f];
            for &a in &float_vals() {
                for &b in &float_vals() {
                    assert_matches_interpreter(
                        4,
                        &[DexValue::Float(a), DexValue::Float(b)],
                        &[RegType::Float, RegType::Float],
                        &method,
                    );
                }
            }
        }
        for opcode in [0x002fu16, 0x0030] {
            let method = vec![opcode, 0x0402, 0x000f];
            for &a in &float_vals() {
                for &b in &float_vals() {
                    assert_matches_interpreter(
                        6,
                        &double_params(a as f64, b as f64),
                        &double_types(),
                        &method,
                    );
                }
            }
        }
    }

    // cite: art entrypoint_utils.h#art_float_to_integral
    // cite: art interpreter_switch_impl-inl.h#INT_TO_FLOAT, INT_TO_DOUBLE, LONG_TO_INT, LONG_TO_FLOAT, LONG_TO_DOUBLE, FLOAT_TO_INT, FLOAT_TO_LONG, FLOAT_TO_DOUBLE, DOUBLE_TO_INT, DOUBLE_TO_LONG, DOUBLE_TO_FLOAT, INT_TO_BYTE, INT_TO_CHAR, INT_TO_SHORT
    #[test]
    fn conversions_match_the_interpreter() {
        #[derive(Clone, Copy)]
        enum Kind {
            Int,
            Long,
            Float,
            Double,
        }
        let int_vals = [
            0i32,
            1,
            -1,
            i32::MIN,
            i32::MAX,
            255,
            256,
            -128,
            65535,
            70000,
        ];
        let long_vals = [0i64, 1, -1, i64::MIN, i64::MAX, 1 << 40, -(1 << 40)];
        let table: &[(u16, Kind, bool)] = &[
            (0x82, Kind::Int, false),
            (0x83, Kind::Int, true),
            (0x84, Kind::Long, false),
            (0x85, Kind::Long, false),
            (0x86, Kind::Long, true),
            (0x87, Kind::Float, false),
            (0x88, Kind::Float, true),
            (0x89, Kind::Float, true),
            (0x8a, Kind::Double, false),
            (0x8b, Kind::Double, true),
            (0x8c, Kind::Double, false),
            (0x8d, Kind::Int, false),
            (0x8e, Kind::Int, false),
            (0x8f, Kind::Int, false),
        ];
        for &(opcode, src, res_wide) in table {
            let ret = if res_wide { 0x0010u16 } else { 0x000f };
            let (src_wide, params, types): (bool, Vec<Vec<DexValue>>, Vec<RegType>) = match src {
                Kind::Int => (
                    false,
                    int_vals.iter().map(|&x| vec![DexValue::Int(x)]).collect(),
                    vec![RegType::Int32],
                ),
                Kind::Long => (
                    true,
                    long_vals
                        .iter()
                        .map(|&x| vec![DexValue::Long(x), DexValue::Long(x)])
                        .collect(),
                    vec![RegType::LongLo, RegType::LongHi],
                ),
                Kind::Float => (
                    false,
                    float_vals()
                        .iter()
                        .map(|&x| vec![DexValue::Float(x)])
                        .collect(),
                    vec![RegType::Float],
                ),
                Kind::Double => (
                    true,
                    float_vals()
                        .iter()
                        .map(|&x| {
                            let bits = (x as f64).to_bits() as i64;
                            vec![DexValue::Long(bits), DexValue::Long(bits)]
                        })
                        .collect(),
                    vec![RegType::DoubleLo, RegType::DoubleHi],
                ),
            };
            let src_reg: u16 = if src_wide { 4 } else { 5 };
            let method = vec![(src_reg << 12) | opcode, ret];
            for param in &params {
                assert_matches_interpreter(6, param, &types, &method);
            }
        }
    }

    // cite: art interpreter_switch_impl-inl.h#MOVE_FROM16, MOVE_OBJECT_FROM16
    #[test]
    fn move_from16_copies_a_param_and_matches_the_interpreter() {
        let method = vec![0x0002, 0x0002, 0x000f];
        for v in [0i32, -1, 42, i32::MIN, i32::MAX] {
            assert_matches_interpreter(3, &[DexValue::Int(v)], &[RegType::Int32], &method);
        }
    }

    fn deopts(registers_size: u16, param_types: &[RegType], instructions: &[u16]) {
        let verified = verify_method(instructions, registers_size, param_types, &[], &Empty)
            .expect("verifies");
        let wir = lower_method(instructions, &verified, &Empty).expect("lowers");
        let mut frame = vec![ExecValue::Reference(1); registers_size as usize];
        let mut heap = Dex3oatHeap::new();
        assert_eq!(execute(&wir, &mut frame, &mut heap), ExecOutcome::Deopt);
    }

    // cite: art interpreter_switch_impl-inl.h#CHECK_CAST, INSTANCE_OF, THROW
    #[test]
    fn check_cast_instance_of_and_throw_deopt_to_the_interpreter() {
        deopts(2, &[RegType::Reference], &[0x011f, 0x0000, 0x000e]);
        deopts(2, &[RegType::Reference], &[0x1020, 0x0000, 0x000f]);
        deopts(2, &[RegType::Reference], &[0x0127]);
    }

    // cite: art interpreter_switch_impl-inl.h#NEG_INT, NOT_INT
    #[test]
    fn neg_and_not_int_match_the_interpreter() {
        let mut state = 0xa5a5_5a5a_a5a5_5a5au64;
        for (opcode, method) in [
            (0x7bu16, vec![0x107b, 0x000f]),
            (0x7c, vec![0x107c, 0x000f]),
        ] {
            let _ = opcode;
            for x in [0i32, 1, -1, i32::MIN, i32::MAX] {
                assert_matches_interpreter(2, &[DexValue::Int(x)], &[RegType::Int32], &method);
            }
            for _ in 0..300 {
                let x = next_random(&mut state);
                assert_matches_interpreter(2, &[DexValue::Int(x)], &[RegType::Int32], &method);
            }
        }
    }

    #[test]
    fn a_method_with_an_invoke_deopts() {
        let _ = DexProto {
            shorty: String::new(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        };
        let method = vec![0x0012, 0x000f];
        assert_matches_interpreter(1, &[], &[], &method);
    }
}

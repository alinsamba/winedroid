use crate::dex3oat::reg_type::RegType;
use crate::dex3oat::verifier::RegisterLine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    AppDirect,
    FrameworkBoundary,
}

// cite: art dex_instruction_list.h#INVOKE_VIRTUAL, INVOKE_SUPER, INVOKE_DIRECT, INVOKE_STATIC, INVOKE_INTERFACE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokeKind {
    Virtual,
    Super,
    Direct,
    Static,
    Interface,
}

// cite: art dex_instruction_list.h#INVOKE_VIRTUAL, INVOKE_VIRTUAL_RANGE
fn invoke_kind(opcode: u8) -> InvokeKind {
    match opcode {
        0x6f | 0x75 => InvokeKind::Super,
        0x70 | 0x76 => InvokeKind::Direct,
        0x71 | 0x77 => InvokeKind::Static,
        0x72 | 0x78 => InvokeKind::Interface,
        _ => InvokeKind::Virtual,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfCmp {
    Eq,
    Ne,
    Lt,
    Ge,
    Gt,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongBinKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftKind {
    Shl,
    Shr,
    Ushr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatBinKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpBias {
    Less,
    Greater,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvKind {
    IntToFloat,
    IntToDouble,
    LongToInt,
    LongToFloat,
    LongToDouble,
    FloatToInt,
    FloatToLong,
    FloatToDouble,
    DoubleToInt,
    DoubleToLong,
    DoubleToFloat,
    IntToByte,
    IntToChar,
    IntToShort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntBinKind {
    Add,
    Sub,
    Rsub,
    Mul,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Ushr,
    Div,
    Rem,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WirOp {
    ConstInt {
        dst: u16,
        value: i32,
    },
    IntBinOp {
        dst: u16,
        a: u16,
        b: u16,
        kind: IntBinKind,
    },
    IntBinLit {
        dst: u16,
        src: u16,
        literal: i32,
        kind: IntBinKind,
    },
    FieldGet {
        dst: u16,
        obj: u16,
        slot: u16,
        ty: RegType,
        field_index: u32,
    },
    FieldSet {
        obj: u16,
        slot: u16,
        src: u16,
        ty: RegType,
        field_index: u32,
    },
    Invoke {
        kind: CallKind,
        dex_kind: InvokeKind,
        method_index: u32,
        args: Vec<u16>,
    },
    Move {
        dst: u16,
        src: u16,
        ty: RegType,
    },
    MoveResult {
        dst: u16,
        ty: RegType,
    },
    Return {
        src: u16,
        ty: RegType,
    },
    ReturnVoid,
    Goto {
        target: usize,
    },
    IfCmp {
        a: u16,
        b: u16,
        cmp: IfCmp,
        target: usize,
    },
    IfZero {
        a: u16,
        cmp: IfCmp,
        target: usize,
    },
    CheckCast {
        reg: u16,
        type_index: u32,
    },
    InstanceOf {
        dst: u16,
        reg: u16,
        type_index: u32,
    },
    Throw {
        reg: u16,
    },
    LongConst {
        dst: u16,
        value: i64,
    },
    LongBinOp {
        dst: u16,
        a: u16,
        b: u16,
        kind: LongBinKind,
    },
    LongShift {
        dst: u16,
        src: u16,
        amount: u16,
        kind: ShiftKind,
    },
    LongNeg {
        dst: u16,
        src: u16,
    },
    LongNot {
        dst: u16,
        src: u16,
    },
    CmpLong {
        dst: u16,
        a: u16,
        b: u16,
    },
    IntToLong {
        dst: u16,
        src: u16,
    },
    FloatBinOp {
        dst: u16,
        a: u16,
        b: u16,
        kind: FloatBinKind,
    },
    DoubleBinOp {
        dst: u16,
        a: u16,
        b: u16,
        kind: FloatBinKind,
    },
    FloatNeg {
        dst: u16,
        src: u16,
    },
    DoubleNeg {
        dst: u16,
        src: u16,
    },
    CmpFloat {
        dst: u16,
        a: u16,
        b: u16,
        bias: CmpBias,
    },
    CmpDouble {
        dst: u16,
        a: u16,
        b: u16,
        bias: CmpBias,
    },
    Convert {
        dst: u16,
        src: u16,
        kind: ConvKind,
    },
    ConstString {
        dst: u16,
        string_index: u32,
    },
    ConstClass {
        dst: u16,
        type_index: u32,
    },
    NewInstance {
        dst: u16,
        type_index: u32,
    },
    StaticGet {
        dst: u16,
        field_index: u32,
        ty: RegType,
    },
    StaticPut {
        src: u16,
        field_index: u32,
    },
    HandlerFieldGet {
        dst: u16,
        obj: u16,
        field_index: u32,
        ty: RegType,
    },
    HandlerFieldSet {
        obj: u16,
        src: u16,
        field_index: u32,
        ty: RegType,
    },
    MoveException {
        dst: u16,
    },
    IntNeg {
        dst: u16,
        src: u16,
    },
    IntNot {
        dst: u16,
        src: u16,
    },
    ArrayLength {
        dst: u16,
        array: u16,
    },
    ArrayGet {
        dst: u16,
        array: u16,
        index: u16,
        elem: ArrayElem,
    },
    ArraySet {
        array: u16,
        index: u16,
        src: u16,
        elem: ArrayElem,
    },
    NewArray {
        dst: u16,
        size: u16,
        component: RegType,
    },
    FilledNewArray {
        type_index: u32,
    },
    FillArrayData {
        array: u16,
    },
    Switch {
        value: u16,
    },
}

impl WirOp {
    // cite: art nodes.cc#InlineInto
    pub fn map_registers(&self, f: impl Fn(u16) -> u16) -> WirOp {
        match self {
            WirOp::ConstInt { dst, value } => WirOp::ConstInt {
                dst: f(*dst),
                value: *value,
            },
            WirOp::IntBinOp { dst, a, b, kind } => WirOp::IntBinOp {
                dst: f(*dst),
                a: f(*a),
                b: f(*b),
                kind: *kind,
            },
            WirOp::IntBinLit {
                dst,
                src,
                literal,
                kind,
            } => WirOp::IntBinLit {
                dst: f(*dst),
                src: f(*src),
                literal: *literal,
                kind: *kind,
            },
            WirOp::FieldGet {
                dst,
                obj,
                slot,
                ty,
                field_index,
            } => WirOp::FieldGet {
                dst: f(*dst),
                obj: f(*obj),
                slot: *slot,
                ty: *ty,
                field_index: *field_index,
            },
            WirOp::FieldSet {
                obj,
                slot,
                src,
                ty,
                field_index,
            } => WirOp::FieldSet {
                obj: f(*obj),
                slot: *slot,
                src: f(*src),
                ty: *ty,
                field_index: *field_index,
            },
            WirOp::Invoke {
                kind,
                dex_kind,
                method_index,
                args,
            } => WirOp::Invoke {
                kind: *kind,
                dex_kind: *dex_kind,
                method_index: *method_index,
                args: args.iter().map(|r| f(*r)).collect(),
            },
            WirOp::Move { dst, src, ty } => WirOp::Move {
                dst: f(*dst),
                src: f(*src),
                ty: *ty,
            },
            WirOp::MoveResult { dst, ty } => WirOp::MoveResult {
                dst: f(*dst),
                ty: *ty,
            },
            WirOp::Return { src, ty } => WirOp::Return {
                src: f(*src),
                ty: *ty,
            },
            WirOp::ReturnVoid => WirOp::ReturnVoid,
            WirOp::Goto { target } => WirOp::Goto { target: *target },
            WirOp::IfCmp { a, b, cmp, target } => WirOp::IfCmp {
                a: f(*a),
                b: f(*b),
                cmp: *cmp,
                target: *target,
            },
            WirOp::IfZero { a, cmp, target } => WirOp::IfZero {
                a: f(*a),
                cmp: *cmp,
                target: *target,
            },
            WirOp::CheckCast { reg, type_index } => WirOp::CheckCast {
                reg: f(*reg),
                type_index: *type_index,
            },
            WirOp::InstanceOf {
                dst,
                reg,
                type_index,
            } => WirOp::InstanceOf {
                dst: f(*dst),
                reg: f(*reg),
                type_index: *type_index,
            },
            WirOp::Throw { reg } => WirOp::Throw { reg: f(*reg) },
            WirOp::LongConst { dst, value } => WirOp::LongConst {
                dst: f(*dst),
                value: *value,
            },
            WirOp::LongBinOp { dst, a, b, kind } => WirOp::LongBinOp {
                dst: f(*dst),
                a: f(*a),
                b: f(*b),
                kind: *kind,
            },
            WirOp::LongShift {
                dst,
                src,
                amount,
                kind,
            } => WirOp::LongShift {
                dst: f(*dst),
                src: f(*src),
                amount: f(*amount),
                kind: *kind,
            },
            WirOp::LongNeg { dst, src } => WirOp::LongNeg {
                dst: f(*dst),
                src: f(*src),
            },
            WirOp::LongNot { dst, src } => WirOp::LongNot {
                dst: f(*dst),
                src: f(*src),
            },
            WirOp::CmpLong { dst, a, b } => WirOp::CmpLong {
                dst: f(*dst),
                a: f(*a),
                b: f(*b),
            },
            WirOp::IntToLong { dst, src } => WirOp::IntToLong {
                dst: f(*dst),
                src: f(*src),
            },
            WirOp::FloatBinOp { dst, a, b, kind } => WirOp::FloatBinOp {
                dst: f(*dst),
                a: f(*a),
                b: f(*b),
                kind: *kind,
            },
            WirOp::DoubleBinOp { dst, a, b, kind } => WirOp::DoubleBinOp {
                dst: f(*dst),
                a: f(*a),
                b: f(*b),
                kind: *kind,
            },
            WirOp::FloatNeg { dst, src } => WirOp::FloatNeg {
                dst: f(*dst),
                src: f(*src),
            },
            WirOp::DoubleNeg { dst, src } => WirOp::DoubleNeg {
                dst: f(*dst),
                src: f(*src),
            },
            WirOp::CmpFloat { dst, a, b, bias } => WirOp::CmpFloat {
                dst: f(*dst),
                a: f(*a),
                b: f(*b),
                bias: *bias,
            },
            WirOp::CmpDouble { dst, a, b, bias } => WirOp::CmpDouble {
                dst: f(*dst),
                a: f(*a),
                b: f(*b),
                bias: *bias,
            },
            WirOp::Convert { dst, src, kind } => WirOp::Convert {
                dst: f(*dst),
                src: f(*src),
                kind: *kind,
            },
            WirOp::ConstString { dst, string_index } => WirOp::ConstString {
                dst: f(*dst),
                string_index: *string_index,
            },
            WirOp::ConstClass { dst, type_index } => WirOp::ConstClass {
                dst: f(*dst),
                type_index: *type_index,
            },
            WirOp::NewInstance { dst, type_index } => WirOp::NewInstance {
                dst: f(*dst),
                type_index: *type_index,
            },
            WirOp::StaticGet {
                dst,
                field_index,
                ty,
            } => WirOp::StaticGet {
                dst: f(*dst),
                field_index: *field_index,
                ty: *ty,
            },
            WirOp::StaticPut { src, field_index } => WirOp::StaticPut {
                src: f(*src),
                field_index: *field_index,
            },
            WirOp::HandlerFieldGet {
                dst,
                obj,
                field_index,
                ty,
            } => WirOp::HandlerFieldGet {
                dst: f(*dst),
                obj: f(*obj),
                field_index: *field_index,
                ty: *ty,
            },
            WirOp::HandlerFieldSet {
                obj,
                src,
                field_index,
                ty,
            } => WirOp::HandlerFieldSet {
                obj: f(*obj),
                src: f(*src),
                field_index: *field_index,
                ty: *ty,
            },
            WirOp::MoveException { dst } => WirOp::MoveException { dst: f(*dst) },
            WirOp::IntNeg { dst, src } => WirOp::IntNeg {
                dst: f(*dst),
                src: f(*src),
            },
            WirOp::IntNot { dst, src } => WirOp::IntNot {
                dst: f(*dst),
                src: f(*src),
            },
            WirOp::ArrayLength { dst, array } => WirOp::ArrayLength {
                dst: f(*dst),
                array: f(*array),
            },
            WirOp::ArrayGet {
                dst,
                array,
                index,
                elem,
            } => WirOp::ArrayGet {
                dst: f(*dst),
                array: f(*array),
                index: f(*index),
                elem: *elem,
            },
            WirOp::ArraySet {
                array,
                index,
                src,
                elem,
            } => WirOp::ArraySet {
                array: f(*array),
                index: f(*index),
                src: f(*src),
                elem: *elem,
            },
            WirOp::NewArray {
                dst,
                size,
                component,
            } => WirOp::NewArray {
                dst: f(*dst),
                size: f(*size),
                component: *component,
            },
            WirOp::FilledNewArray { type_index } => WirOp::FilledNewArray {
                type_index: *type_index,
            },
            WirOp::FillArrayData { array } => WirOp::FillArrayData { array: f(*array) },
            WirOp::Switch { value } => WirOp::Switch { value: f(*value) },
        }
    }

    // cite: art nodes.h#GetType
    pub fn defined_register(&self) -> Option<u16> {
        match self {
            WirOp::ConstInt { dst, .. }
            | WirOp::IntBinOp { dst, .. }
            | WirOp::IntBinLit { dst, .. }
            | WirOp::FieldGet { dst, .. }
            | WirOp::Move { dst, .. }
            | WirOp::MoveResult { dst, .. }
            | WirOp::InstanceOf { dst, .. }
            | WirOp::LongConst { dst, .. }
            | WirOp::LongBinOp { dst, .. }
            | WirOp::LongShift { dst, .. }
            | WirOp::LongNeg { dst, .. }
            | WirOp::LongNot { dst, .. }
            | WirOp::CmpLong { dst, .. }
            | WirOp::IntToLong { dst, .. }
            | WirOp::FloatBinOp { dst, .. }
            | WirOp::DoubleBinOp { dst, .. }
            | WirOp::FloatNeg { dst, .. }
            | WirOp::DoubleNeg { dst, .. }
            | WirOp::CmpFloat { dst, .. }
            | WirOp::CmpDouble { dst, .. }
            | WirOp::Convert { dst, .. }
            | WirOp::ConstString { dst, .. }
            | WirOp::ConstClass { dst, .. }
            | WirOp::NewInstance { dst, .. }
            | WirOp::StaticGet { dst, .. }
            | WirOp::HandlerFieldGet { dst, .. }
            | WirOp::MoveException { dst, .. }
            | WirOp::IntNeg { dst, .. }
            | WirOp::IntNot { dst, .. }
            | WirOp::ArrayLength { dst, .. }
            | WirOp::ArrayGet { dst, .. }
            | WirOp::NewArray { dst, .. } => Some(*dst),
            WirOp::FieldSet { .. }
            | WirOp::Invoke { .. }
            | WirOp::Return { .. }
            | WirOp::ReturnVoid
            | WirOp::Goto { .. }
            | WirOp::IfCmp { .. }
            | WirOp::IfZero { .. }
            | WirOp::CheckCast { .. }
            | WirOp::Throw { .. }
            | WirOp::StaticPut { .. }
            | WirOp::HandlerFieldSet { .. }
            | WirOp::ArraySet { .. }
            | WirOp::FilledNewArray { .. }
            | WirOp::FillArrayData { .. }
            | WirOp::Switch { .. } => None,
        }
    }

    // cite: art nodes.h#GetInputs
    pub fn used_registers(&self) -> Vec<u16> {
        match self {
            WirOp::ConstInt { .. }
            | WirOp::ReturnVoid
            | WirOp::Goto { .. }
            | WirOp::LongConst { .. }
            | WirOp::ConstString { .. }
            | WirOp::ConstClass { .. }
            | WirOp::NewInstance { .. }
            | WirOp::StaticGet { .. }
            | WirOp::MoveException { .. }
            | WirOp::MoveResult { .. }
            | WirOp::FilledNewArray { .. } => Vec::new(),
            WirOp::IntBinOp { a, b, .. }
            | WirOp::IfCmp { a, b, .. }
            | WirOp::LongBinOp { a, b, .. }
            | WirOp::CmpLong { a, b, .. }
            | WirOp::FloatBinOp { a, b, .. }
            | WirOp::DoubleBinOp { a, b, .. }
            | WirOp::CmpFloat { a, b, .. }
            | WirOp::CmpDouble { a, b, .. } => vec![*a, *b],
            WirOp::IntBinLit { src, .. }
            | WirOp::Move { src, .. }
            | WirOp::Return { src, .. }
            | WirOp::LongNeg { src, .. }
            | WirOp::LongNot { src, .. }
            | WirOp::IntToLong { src, .. }
            | WirOp::FloatNeg { src, .. }
            | WirOp::DoubleNeg { src, .. }
            | WirOp::Convert { src, .. }
            | WirOp::StaticPut { src, .. }
            | WirOp::IntNeg { src, .. }
            | WirOp::IntNot { src, .. } => vec![*src],
            WirOp::IfZero { a, .. } => vec![*a],
            WirOp::CheckCast { reg, .. }
            | WirOp::InstanceOf { reg, .. }
            | WirOp::Throw { reg, .. } => vec![*reg],
            WirOp::FieldGet { obj, .. } | WirOp::HandlerFieldGet { obj, .. } => vec![*obj],
            WirOp::FieldSet { obj, src, .. } | WirOp::HandlerFieldSet { obj, src, .. } => {
                vec![*obj, *src]
            }
            WirOp::Invoke { args, .. } => args.clone(),
            WirOp::LongShift { src, amount, .. } => vec![*src, *amount],
            WirOp::ArrayLength { array, .. } | WirOp::FillArrayData { array, .. } => vec![*array],
            WirOp::ArrayGet { array, index, .. } => vec![*array, *index],
            WirOp::ArraySet {
                array, index, src, ..
            } => vec![*array, *index, *src],
            WirOp::NewArray { size, .. } => vec![*size],
            WirOp::Switch { value } => vec![*value],
        }
    }

    // cite: art nodes.h#HasSideEffects, CanThrow
    pub fn is_pure(&self) -> bool {
        match self {
            WirOp::ConstInt { .. }
            | WirOp::Move { .. }
            | WirOp::LongConst { .. }
            | WirOp::IntNeg { .. }
            | WirOp::IntNot { .. }
            | WirOp::LongNeg { .. }
            | WirOp::LongNot { .. }
            | WirOp::LongShift { .. }
            | WirOp::CmpLong { .. }
            | WirOp::IntToLong { .. }
            | WirOp::FloatNeg { .. }
            | WirOp::DoubleNeg { .. }
            | WirOp::CmpFloat { .. }
            | WirOp::CmpDouble { .. }
            | WirOp::Convert { .. }
            | WirOp::FloatBinOp { .. }
            | WirOp::DoubleBinOp { .. } => true,
            WirOp::IntBinOp { kind, .. } | WirOp::IntBinLit { kind, .. } => {
                !matches!(kind, IntBinKind::Div | IntBinKind::Rem)
            }
            WirOp::LongBinOp { kind, .. } => !matches!(kind, LongBinKind::Div | LongBinKind::Rem),
            _ => false,
        }
    }

    // cite: art block_builder.cc#CreateBranchTargets
    pub fn retarget(&self, f: impl Fn(usize) -> usize) -> WirOp {
        match self {
            WirOp::Goto { target } => WirOp::Goto { target: f(*target) },
            WirOp::IfCmp { a, b, cmp, target } => WirOp::IfCmp {
                a: *a,
                b: *b,
                cmp: *cmp,
                target: f(*target),
            },
            WirOp::IfZero { a, cmp, target } => WirOp::IfZero {
                a: *a,
                cmp: *cmp,
                target: f(*target),
            },
            other => other.clone(),
        }
    }
}

// cite: art dex_instruction_list.h#AGET, AGET_WIDE, AGET_OBJECT, AGET_BOOLEAN, AGET_BYTE, AGET_CHAR, AGET_SHORT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayElem {
    Int,
    Wide,
    Object,
    Boolean,
    Byte,
    Char,
    Short,
}

// cite: art dex_instruction_list.h#AGET, APUT
fn array_elem(opcode: u8) -> ArrayElem {
    let base = if (0x44..=0x4a).contains(&opcode) {
        opcode - 0x44
    } else {
        opcode - 0x4b
    };
    match base {
        1 => ArrayElem::Wide,
        2 => ArrayElem::Object,
        3 => ArrayElem::Boolean,
        4 => ArrayElem::Byte,
        5 => ArrayElem::Char,
        6 => ArrayElem::Short,
        _ => ArrayElem::Int,
    }
}

// cite: art dex_instruction_list.h#INVOKE_DIRECT, INVOKE_STATIC, INVOKE_DIRECT_RANGE, INVOKE_STATIC_RANGE
fn invoke_is_inlinable(opcode: u8) -> bool {
    matches!(opcode, 0x70 | 0x71 | 0x76 | 0x77)
}

pub trait WirResolver {
    fn field_slot_and_type(&self, field_index: u32) -> Option<(u16, RegType)>;
    fn call_kind(&self, method_index: u32) -> Option<CallKind>;
    // cite: art reg_type_cache.cc#FromDescriptor
    fn array_component(&self, type_index: u32) -> Option<RegType>;
    // cite: art reg_type_cache.cc#FromDescriptor
    fn field_type(&self, field_index: u32) -> Option<RegType>;
}

fn type_at(verified: &[Option<RegisterLine>], pc: usize, reg: usize) -> Option<RegType> {
    verified.get(pc)?.as_ref()?.get(reg).copied()
}

// cite: art dex_instruction_list.h#ADD_INT, SUB_INT, MUL_INT, DIV_INT, REM_INT, AND_INT, OR_INT, XOR_INT, SHL_INT, SHR_INT, USHR_INT
fn int_bin_kind(opcode: u8) -> Option<IntBinKind> {
    let base = if (0x90..=0x9a).contains(&opcode) {
        opcode - 0x90
    } else if (0xb0..=0xba).contains(&opcode) {
        opcode - 0xb0
    } else {
        return None;
    };
    Some(match base {
        0 => IntBinKind::Add,
        1 => IntBinKind::Sub,
        2 => IntBinKind::Mul,
        3 => IntBinKind::Div,
        4 => IntBinKind::Rem,
        5 => IntBinKind::And,
        6 => IntBinKind::Or,
        7 => IntBinKind::Xor,
        8 => IntBinKind::Shl,
        9 => IntBinKind::Shr,
        10 => IntBinKind::Ushr,
        _ => return None,
    })
}

// cite: art dex_instruction_list.h#ADD_INT_LIT16, RSUB_INT, MUL_INT_LIT16, ADD_INT_LIT8, RSUB_INT_LIT8, SHL_INT_LIT8
fn lit_bin_kind(opcode: u8) -> Option<IntBinKind> {
    let base = if (0xd0..=0xd7).contains(&opcode) {
        opcode - 0xd0
    } else if (0xd8..=0xe2).contains(&opcode) {
        opcode - 0xd8
    } else {
        return None;
    };
    Some(match base {
        0 => IntBinKind::Add,
        1 => IntBinKind::Rsub,
        2 => IntBinKind::Mul,
        3 => IntBinKind::Div,
        4 => IntBinKind::Rem,
        5 => IntBinKind::And,
        6 => IntBinKind::Or,
        7 => IntBinKind::Xor,
        8 => IntBinKind::Shl,
        9 => IntBinKind::Shr,
        10 => IntBinKind::Ushr,
        _ => return None,
    })
}

// cite: art dex_instruction_list.h#ADD_LONG, SUB_LONG, MUL_LONG, DIV_LONG, REM_LONG, AND_LONG, OR_LONG, XOR_LONG
fn long_bin_kind(opcode: u8) -> Option<LongBinKind> {
    let base = if (0x9b..=0xa2).contains(&opcode) {
        opcode - 0x9b
    } else if (0xbb..=0xc2).contains(&opcode) {
        opcode - 0xbb
    } else {
        return None;
    };
    Some(match base {
        0 => LongBinKind::Add,
        1 => LongBinKind::Sub,
        2 => LongBinKind::Mul,
        3 => LongBinKind::Div,
        4 => LongBinKind::Rem,
        5 => LongBinKind::And,
        6 => LongBinKind::Or,
        7 => LongBinKind::Xor,
        _ => return None,
    })
}

// cite: art dex_instruction_list.h#SHL_LONG, SHR_LONG, USHR_LONG
fn long_shift_kind(opcode: u8) -> Option<ShiftKind> {
    let base = if (0xa3..=0xa5).contains(&opcode) {
        opcode - 0xa3
    } else if (0xc3..=0xc5).contains(&opcode) {
        opcode - 0xc3
    } else {
        return None;
    };
    Some(match base {
        0 => ShiftKind::Shl,
        1 => ShiftKind::Shr,
        2 => ShiftKind::Ushr,
        _ => return None,
    })
}

// cite: art dex_instruction_list.h#ADD_FLOAT, SUB_FLOAT, MUL_FLOAT, DIV_FLOAT, REM_FLOAT, ADD_DOUBLE, SUB_DOUBLE, MUL_DOUBLE, DIV_DOUBLE, REM_DOUBLE
fn float_bin_kind(opcode: u8) -> Option<FloatBinKind> {
    let base = if (0xa6..=0xaa).contains(&opcode) {
        opcode - 0xa6
    } else if (0xab..=0xaf).contains(&opcode) {
        opcode - 0xab
    } else if (0xc6..=0xca).contains(&opcode) {
        opcode - 0xc6
    } else if (0xcb..=0xcf).contains(&opcode) {
        opcode - 0xcb
    } else {
        return None;
    };
    Some(match base {
        0 => FloatBinKind::Add,
        1 => FloatBinKind::Sub,
        2 => FloatBinKind::Mul,
        3 => FloatBinKind::Div,
        4 => FloatBinKind::Rem,
        _ => return None,
    })
}

// cite: art dex_instruction_list.h#INT_TO_FLOAT, INT_TO_DOUBLE, LONG_TO_INT, LONG_TO_FLOAT, LONG_TO_DOUBLE, FLOAT_TO_INT, FLOAT_TO_LONG, FLOAT_TO_DOUBLE, DOUBLE_TO_INT, DOUBLE_TO_LONG, DOUBLE_TO_FLOAT, INT_TO_BYTE, INT_TO_CHAR, INT_TO_SHORT
fn conv_kind(opcode: u8) -> Option<ConvKind> {
    Some(match opcode {
        0x82 => ConvKind::IntToFloat,
        0x83 => ConvKind::IntToDouble,
        0x84 => ConvKind::LongToInt,
        0x85 => ConvKind::LongToFloat,
        0x86 => ConvKind::LongToDouble,
        0x87 => ConvKind::FloatToInt,
        0x88 => ConvKind::FloatToLong,
        0x89 => ConvKind::FloatToDouble,
        0x8a => ConvKind::DoubleToInt,
        0x8b => ConvKind::DoubleToLong,
        0x8c => ConvKind::DoubleToFloat,
        0x8d => ConvKind::IntToByte,
        0x8e => ConvKind::IntToChar,
        0x8f => ConvKind::IntToShort,
        _ => return None,
    })
}

pub fn lower_method(
    instructions: &[u16],
    verified: &[Option<RegisterLine>],
    resolver: &dyn WirResolver,
) -> Option<Vec<(usize, WirOp)>> {
    let mut lowered = Vec::new();
    for pc in 0..instructions.len() {
        if verified.get(pc).and_then(|line| line.as_ref()).is_none() {
            continue;
        }
        let insn = instructions[pc];
        let opcode = (insn & 0x00ff) as u8;
        let a4 = ((insn >> 8) & 0x0f) as u16;
        let b4 = ((insn >> 12) & 0x0f) as u16;
        let a8 = ((insn >> 8) & 0xff) as u16;

        match opcode {
            0x12 => {
                let value = ((insn as i16) >> 12) as i32;
                lowered.push((pc, WirOp::ConstInt { dst: a4, value }));
            }
            0x13 => {
                let value = *instructions.get(pc + 1)? as i16 as i32;
                lowered.push((pc, WirOp::ConstInt { dst: a8, value }));
            }
            0x90..=0x9a => {
                let bc = *instructions.get(pc + 1)?;
                lowered.push((
                    pc,
                    WirOp::IntBinOp {
                        dst: a8,
                        a: bc & 0xff,
                        b: (bc >> 8) & 0xff,
                        kind: int_bin_kind(opcode)?,
                    },
                ));
            }
            0xb0..=0xba => {
                lowered.push((
                    pc,
                    WirOp::IntBinOp {
                        dst: a4,
                        a: a4,
                        b: b4,
                        kind: int_bin_kind(opcode)?,
                    },
                ));
            }
            0xd0..=0xd7 => {
                let literal = *instructions.get(pc + 1)? as i16 as i32;
                lowered.push((
                    pc,
                    WirOp::IntBinLit {
                        dst: a4,
                        src: b4,
                        literal,
                        kind: lit_bin_kind(opcode)?,
                    },
                ));
            }
            0xd8..=0xe2 => {
                let unit = *instructions.get(pc + 1)?;
                let src = unit & 0xff;
                let literal = ((unit >> 8) as u8 as i8) as i32;
                lowered.push((
                    pc,
                    WirOp::IntBinLit {
                        dst: a8,
                        src,
                        literal,
                        kind: lit_bin_kind(opcode)?,
                    },
                ));
            }
            0x52..=0x58 => {
                let field_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    match resolver.field_slot_and_type(field_index) {
                        Some((slot, ty)) => WirOp::FieldGet {
                            dst: a4,
                            obj: b4,
                            slot,
                            ty,
                            field_index,
                        },
                        None => WirOp::HandlerFieldGet {
                            dst: a4,
                            obj: b4,
                            field_index,
                            ty: resolver.field_type(field_index)?,
                        },
                    },
                ));
            }
            0x59..=0x5f => {
                let field_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    match resolver.field_slot_and_type(field_index) {
                        Some((slot, ty)) => WirOp::FieldSet {
                            obj: b4,
                            slot,
                            src: a4,
                            ty,
                            field_index,
                        },
                        None => WirOp::HandlerFieldSet {
                            obj: b4,
                            src: a4,
                            field_index,
                            ty: resolver.field_type(field_index)?,
                        },
                    },
                ));
            }
            0x6e..=0x72 => {
                let method_index = *instructions.get(pc + 1)? as u32;
                let kind = if invoke_is_inlinable(opcode) {
                    resolver.call_kind(method_index)?
                } else {
                    CallKind::FrameworkBoundary
                };
                let count = ((insn >> 12) & 0x0f) as usize;
                let regs = *instructions.get(pc + 2)?;
                let candidates = [
                    regs & 0x0f,
                    (regs >> 4) & 0x0f,
                    (regs >> 8) & 0x0f,
                    (regs >> 12) & 0x0f,
                    (insn >> 8) & 0x0f,
                ];
                let args = candidates.into_iter().take(count).collect();
                lowered.push((
                    pc,
                    WirOp::Invoke {
                        kind,
                        dex_kind: invoke_kind(opcode),
                        method_index,
                        args,
                    },
                ));
            }
            0x74..=0x78 => {
                let method_index = *instructions.get(pc + 1)? as u32;
                let kind = if invoke_is_inlinable(opcode) {
                    resolver.call_kind(method_index)?
                } else {
                    CallKind::FrameworkBoundary
                };
                let count = ((insn >> 8) & 0xff) as u16;
                let first = *instructions.get(pc + 2)?;
                let args = (0..count).map(|offset| first + offset).collect();
                lowered.push((
                    pc,
                    WirOp::Invoke {
                        kind,
                        dex_kind: invoke_kind(opcode),
                        method_index,
                        args,
                    },
                ));
            }
            0x0a | 0x0b | 0x0c => {
                let result_slot = verified[pc].as_ref()?.len() - 1;
                let ty = type_at(verified, pc, result_slot)?;
                lowered.push((pc, WirOp::MoveResult { dst: a8, ty }));
            }
            0x0f | 0x10 | 0x11 => {
                let ty = type_at(verified, pc, a8 as usize)?;
                // cite: art dex_instruction_list.h#RETURN, RETURN_OBJECT
                let ty = match opcode {
                    0x11 => RegType::Reference,
                    0x0f if ty == RegType::Zero || ty == RegType::Const32 => RegType::Int32,
                    _ => ty,
                };
                lowered.push((pc, WirOp::Return { src: a8, ty }));
            }
            0x0e => lowered.push((pc, WirOp::ReturnVoid)),
            0x00 | 0x1d | 0x1e => {}
            0x02 => {
                let src = *instructions.get(pc + 1)?;
                let ty = type_at(verified, pc, src as usize)?;
                lowered.push((pc, WirOp::Move { dst: a8, src, ty }));
            }
            0x03 => {
                let dst = *instructions.get(pc + 1)?;
                let src = *instructions.get(pc + 2)?;
                let ty = type_at(verified, pc, src as usize)?;
                lowered.push((pc, WirOp::Move { dst, src, ty }));
            }
            0x05 | 0x08 => {
                let src = *instructions.get(pc + 1)?;
                let ty = type_at(verified, pc, src as usize)?;
                lowered.push((pc, WirOp::Move { dst: a8, src, ty }));
            }
            0x06 | 0x09 => {
                let dst = *instructions.get(pc + 1)?;
                let src = *instructions.get(pc + 2)?;
                let ty = type_at(verified, pc, src as usize)?;
                lowered.push((pc, WirOp::Move { dst, src, ty }));
            }
            0x1f => {
                let type_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    WirOp::CheckCast {
                        reg: a8,
                        type_index,
                    },
                ));
            }
            0x20 => {
                let type_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    WirOp::InstanceOf {
                        dst: a4,
                        reg: b4,
                        type_index,
                    },
                ));
            }
            0x27 => lowered.push((pc, WirOp::Throw { reg: a8 })),
            0x0d => lowered.push((pc, WirOp::MoveException { dst: a8 })),
            0x7b => lowered.push((pc, WirOp::IntNeg { dst: a4, src: b4 })),
            0x7c => lowered.push((pc, WirOp::IntNot { dst: a4, src: b4 })),
            0x21 => lowered.push((pc, WirOp::ArrayLength { dst: a4, array: b4 })),
            0x23 => {
                let type_index = *instructions.get(pc + 1)? as u32;
                let component = resolver
                    .array_component(type_index)
                    .unwrap_or(RegType::Reference);
                lowered.push((
                    pc,
                    WirOp::NewArray {
                        dst: a4,
                        size: b4,
                        component,
                    },
                ));
            }
            0x24 | 0x25 => {
                let type_index = *instructions.get(pc + 1)? as u32;
                lowered.push((pc, WirOp::FilledNewArray { type_index }));
            }
            0x26 => lowered.push((pc, WirOp::FillArrayData { array: a8 })),
            0x2b | 0x2c => lowered.push((pc, WirOp::Switch { value: a8 })),
            0x44..=0x4a => {
                let unit = *instructions.get(pc + 1)?;
                lowered.push((
                    pc,
                    WirOp::ArrayGet {
                        dst: a8,
                        array: unit & 0xff,
                        index: (unit >> 8) & 0xff,
                        elem: array_elem(opcode),
                    },
                ));
            }
            0x4b..=0x51 => {
                let unit = *instructions.get(pc + 1)?;
                lowered.push((
                    pc,
                    WirOp::ArraySet {
                        array: unit & 0xff,
                        index: (unit >> 8) & 0xff,
                        src: a8,
                        elem: array_elem(opcode),
                    },
                ));
            }
            0x16 => {
                let value = *instructions.get(pc + 1)? as i16 as i64;
                lowered.push((pc, WirOp::LongConst { dst: a8, value }));
            }
            0x17 => {
                let low = *instructions.get(pc + 1)? as u32;
                let high = *instructions.get(pc + 2)? as u32;
                let value = (low | (high << 16)) as i32 as i64;
                lowered.push((pc, WirOp::LongConst { dst: a8, value }));
            }
            0x18 => {
                let w0 = *instructions.get(pc + 1)? as u64;
                let w1 = *instructions.get(pc + 2)? as u64;
                let w2 = *instructions.get(pc + 3)? as u64;
                let w3 = *instructions.get(pc + 4)? as u64;
                let value = (w0 | (w1 << 16) | (w2 << 32) | (w3 << 48)) as i64;
                lowered.push((pc, WirOp::LongConst { dst: a8, value }));
            }
            0x19 => {
                let value = (*instructions.get(pc + 1)? as i16 as i64) << 48;
                lowered.push((pc, WirOp::LongConst { dst: a8, value }));
            }
            0x9b..=0xa2 => {
                let unit = *instructions.get(pc + 1)?;
                lowered.push((
                    pc,
                    WirOp::LongBinOp {
                        dst: a8,
                        a: unit & 0xff,
                        b: (unit >> 8) & 0xff,
                        kind: long_bin_kind(opcode)?,
                    },
                ));
            }
            0xbb..=0xc2 => {
                lowered.push((
                    pc,
                    WirOp::LongBinOp {
                        dst: a4,
                        a: a4,
                        b: b4,
                        kind: long_bin_kind(opcode)?,
                    },
                ));
            }
            0xa3..=0xa5 => {
                let unit = *instructions.get(pc + 1)?;
                lowered.push((
                    pc,
                    WirOp::LongShift {
                        dst: a8,
                        src: unit & 0xff,
                        amount: (unit >> 8) & 0xff,
                        kind: long_shift_kind(opcode)?,
                    },
                ));
            }
            0xc3..=0xc5 => {
                lowered.push((
                    pc,
                    WirOp::LongShift {
                        dst: a4,
                        src: a4,
                        amount: b4,
                        kind: long_shift_kind(opcode)?,
                    },
                ));
            }
            0x7d => lowered.push((pc, WirOp::LongNeg { dst: a4, src: b4 })),
            0x7e => lowered.push((pc, WirOp::LongNot { dst: a4, src: b4 })),
            0x31 => {
                let unit = *instructions.get(pc + 1)?;
                lowered.push((
                    pc,
                    WirOp::CmpLong {
                        dst: a8,
                        a: unit & 0xff,
                        b: (unit >> 8) & 0xff,
                    },
                ));
            }
            0x81 => lowered.push((pc, WirOp::IntToLong { dst: a4, src: b4 })),
            0xa6..=0xaf => {
                let unit = *instructions.get(pc + 1)?;
                let dst = a8;
                let a = unit & 0xff;
                let b = (unit >> 8) & 0xff;
                let kind = float_bin_kind(opcode)?;
                lowered.push((
                    pc,
                    if opcode <= 0xaa {
                        WirOp::FloatBinOp { dst, a, b, kind }
                    } else {
                        WirOp::DoubleBinOp { dst, a, b, kind }
                    },
                ));
            }
            0xc6..=0xcf => {
                let kind = float_bin_kind(opcode)?;
                lowered.push((
                    pc,
                    if opcode <= 0xca {
                        WirOp::FloatBinOp {
                            dst: a4,
                            a: a4,
                            b: b4,
                            kind,
                        }
                    } else {
                        WirOp::DoubleBinOp {
                            dst: a4,
                            a: a4,
                            b: b4,
                            kind,
                        }
                    },
                ));
            }
            0x7f => lowered.push((pc, WirOp::FloatNeg { dst: a4, src: b4 })),
            0x80 => lowered.push((pc, WirOp::DoubleNeg { dst: a4, src: b4 })),
            0x2d..=0x30 => {
                let unit = *instructions.get(pc + 1)?;
                let a = unit & 0xff;
                let b = (unit >> 8) & 0xff;
                let bias = if opcode == 0x2d || opcode == 0x2f {
                    CmpBias::Less
                } else {
                    CmpBias::Greater
                };
                lowered.push((
                    pc,
                    if opcode <= 0x2e {
                        WirOp::CmpFloat {
                            dst: a8,
                            a,
                            b,
                            bias,
                        }
                    } else {
                        WirOp::CmpDouble {
                            dst: a8,
                            a,
                            b,
                            bias,
                        }
                    },
                ));
            }
            0x82..=0x8f => lowered.push((
                pc,
                WirOp::Convert {
                    dst: a4,
                    src: b4,
                    kind: conv_kind(opcode)?,
                },
            )),
            0x1a => {
                let string_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    WirOp::ConstString {
                        dst: a8,
                        string_index,
                    },
                ));
            }
            0x1b => {
                let low = *instructions.get(pc + 1)? as u32;
                let high = *instructions.get(pc + 2)? as u32;
                lowered.push((
                    pc,
                    WirOp::ConstString {
                        dst: a8,
                        string_index: low | (high << 16),
                    },
                ));
            }
            0x1c => {
                let type_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    WirOp::ConstClass {
                        dst: a8,
                        type_index,
                    },
                ));
            }
            0x22 => {
                let type_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    WirOp::NewInstance {
                        dst: a8,
                        type_index,
                    },
                ));
            }
            0x60..=0x66 => {
                let field_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    WirOp::StaticGet {
                        dst: a8,
                        field_index,
                        ty: resolver.field_type(field_index)?,
                    },
                ));
            }
            0x67..=0x6d => {
                let field_index = *instructions.get(pc + 1)? as u32;
                lowered.push((
                    pc,
                    WirOp::StaticPut {
                        src: a8,
                        field_index,
                    },
                ));
            }
            0x01 | 0x04 | 0x07 => {
                let ty = type_at(verified, pc, b4 as usize)?;
                lowered.push((
                    pc,
                    WirOp::Move {
                        dst: a4,
                        src: b4,
                        ty,
                    },
                ));
            }
            0x14 => {
                let low = *instructions.get(pc + 1)? as i32;
                let high = *instructions.get(pc + 2)? as i32;
                lowered.push((
                    pc,
                    WirOp::ConstInt {
                        dst: a8,
                        value: low | (high << 16),
                    },
                ));
            }
            0x15 => {
                let value = (*instructions.get(pc + 1)? as i32) << 16;
                lowered.push((pc, WirOp::ConstInt { dst: a8, value }));
            }
            0x28 => {
                let offset = (insn >> 8) as u8 as i8 as i32;
                lowered.push((
                    pc,
                    WirOp::Goto {
                        target: branch_target(pc, offset)?,
                    },
                ));
            }
            0x29 => {
                let offset = *instructions.get(pc + 1)? as i16 as i32;
                lowered.push((
                    pc,
                    WirOp::Goto {
                        target: branch_target(pc, offset)?,
                    },
                ));
            }
            0x32..=0x37 => {
                let offset = *instructions.get(pc + 1)? as i16 as i32;
                lowered.push((
                    pc,
                    WirOp::IfCmp {
                        a: a4,
                        b: b4,
                        cmp: if_cmp(opcode, 0x32)?,
                        target: branch_target(pc, offset)?,
                    },
                ));
            }
            0x38..=0x3d => {
                let offset = *instructions.get(pc + 1)? as i16 as i32;
                lowered.push((
                    pc,
                    WirOp::IfZero {
                        a: a8,
                        cmp: if_cmp(opcode, 0x38)?,
                        target: branch_target(pc, offset)?,
                    },
                ));
            }
            _ => return None,
        }
    }
    Some(lowered)
}

fn branch_target(pc: usize, offset: i32) -> Option<usize> {
    let target = pc as i64 + offset as i64;
    (target >= 0).then_some(target as usize)
}

// cite: art dex_instruction_list.h#IF_EQ, IF_NE, IF_LT, IF_GE, IF_GT, IF_LE
fn if_cmp(opcode: u8, base: u8) -> Option<IfCmp> {
    Some(match opcode - base {
        0 => IfCmp::Eq,
        1 => IfCmp::Ne,
        2 => IfCmp::Lt,
        3 => IfCmp::Ge,
        4 => IfCmp::Gt,
        5 => IfCmp::Le,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex3oat::verifier::{verify_method, ReturnType, VerifierTypes};

    struct Stub;

    impl VerifierTypes for Stub {
        fn field_type(&self, field_index: u32) -> Option<RegType> {
            match field_index {
                0 => Some(RegType::Int32),
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

    impl WirResolver for Stub {
        fn field_slot_and_type(&self, field_index: u32) -> Option<(u16, RegType)> {
            match field_index {
                0 => Some((5, RegType::Int32)),
                _ => None,
            }
        }
        fn call_kind(&self, method_index: u32) -> Option<CallKind> {
            match method_index {
                0 => Some(CallKind::AppDirect),
                1 => Some(CallKind::FrameworkBoundary),
                _ => None,
            }
        }
        fn array_component(&self, _type_index: u32) -> Option<RegType> {
            Some(RegType::Int32)
        }
        fn field_type(&self, field_index: u32) -> Option<RegType> {
            match field_index {
                0 => Some(RegType::Int32),
                _ => None,
            }
        }
    }

    // cite: art class_linker.cc#LinkFields
    #[test]
    fn iget_lowers_to_a_slot_resolved_typed_field_get() {
        let instructions = [0x1052u16, 0x0000, 0x000f];
        let verified = verify_method(&instructions, 2, &[RegType::Reference], &[], &Stub).unwrap();
        let wir = lower_method(&instructions, &verified, &Stub).unwrap();
        assert_eq!(
            wir[0],
            (
                0,
                WirOp::FieldGet {
                    dst: 0,
                    obj: 1,
                    slot: 5,
                    ty: RegType::Int32,
                    field_index: 0,
                }
            )
        );
        assert_eq!(
            wir[1],
            (
                2,
                WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                }
            )
        );
    }

    // cite: art class_linker.cc#AssignVTableIndexes
    #[test]
    fn an_app_direct_invoke_and_its_result_are_typed_and_classified() {
        let instructions = [0x0071u16, 0x0000, 0x0000, 0x000a, 0x000f];
        let verified = verify_method(&instructions, 1, &[], &[], &Stub).unwrap();
        let wir = lower_method(&instructions, &verified, &Stub).unwrap();
        assert_eq!(
            wir[0],
            (
                0,
                WirOp::Invoke {
                    kind: CallKind::AppDirect,
                    dex_kind: crate::dex3oat::wir::InvokeKind::Static,
                    method_index: 0,
                    args: vec![],
                }
            )
        );
        assert_eq!(
            wir[1],
            (
                3,
                WirOp::MoveResult {
                    dst: 0,
                    ty: RegType::Int32,
                }
            )
        );
    }

    #[test]
    fn lowering_skips_unreachable_operand_code_units() {
        let instructions = [0x0071u16, 0x0001, 0x0000, 0x000e];
        let verified = verify_method(&instructions, 1, &[], &[], &Stub).unwrap();
        let wir = lower_method(&instructions, &verified, &Stub).unwrap();
        assert_eq!(
            wir[0],
            (
                0,
                WirOp::Invoke {
                    kind: CallKind::FrameworkBoundary,
                    dex_kind: crate::dex3oat::wir::InvokeKind::Static,
                    method_index: 1,
                    args: vec![],
                }
            )
        );
        assert_eq!(wir[1], (3, WirOp::ReturnVoid));
        assert_eq!(wir.len(), 2);
    }
}

use crate::dex3oat::reg_type::RegType;
use crate::dex3oat::wir::{CallKind, WirOp};

pub struct InlinableCallee {
    pub ops: Vec<WirOp>,
    pub param_regs: Vec<u16>,
    pub param_types: Vec<RegType>,
    pub register_count: u16,
}

pub trait CalleeProvider {
    fn callee(&self, method_index: u32) -> Option<InlinableCallee>;
}

fn shift(reg: u16, base: u16) -> Option<u16> {
    reg.checked_add(base)
}

fn remap(op: &WirOp, base: u16) -> Option<WirOp> {
    Some(match op {
        WirOp::ConstInt { dst, value } => WirOp::ConstInt {
            dst: shift(*dst, base)?,
            value: *value,
        },
        WirOp::IntBinOp { dst, a, b, kind } => WirOp::IntBinOp {
            dst: shift(*dst, base)?,
            a: shift(*a, base)?,
            b: shift(*b, base)?,
            kind: *kind,
        },
        WirOp::IntBinLit {
            dst,
            src,
            literal,
            kind,
        } => WirOp::IntBinLit {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
            literal: *literal,
            kind: *kind,
        },
        WirOp::LongConst { dst, value } => WirOp::LongConst {
            dst: shift(*dst, base)?,
            value: *value,
        },
        WirOp::LongBinOp { dst, a, b, kind } => WirOp::LongBinOp {
            dst: shift(*dst, base)?,
            a: shift(*a, base)?,
            b: shift(*b, base)?,
            kind: *kind,
        },
        WirOp::LongShift {
            dst,
            src,
            amount,
            kind,
        } => WirOp::LongShift {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
            amount: shift(*amount, base)?,
            kind: *kind,
        },
        WirOp::LongNeg { dst, src } => WirOp::LongNeg {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
        },
        WirOp::LongNot { dst, src } => WirOp::LongNot {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
        },
        WirOp::CmpLong { dst, a, b } => WirOp::CmpLong {
            dst: shift(*dst, base)?,
            a: shift(*a, base)?,
            b: shift(*b, base)?,
        },
        WirOp::IntToLong { dst, src } => WirOp::IntToLong {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
        },
        WirOp::FloatBinOp { dst, a, b, kind } => WirOp::FloatBinOp {
            dst: shift(*dst, base)?,
            a: shift(*a, base)?,
            b: shift(*b, base)?,
            kind: *kind,
        },
        WirOp::DoubleBinOp { dst, a, b, kind } => WirOp::DoubleBinOp {
            dst: shift(*dst, base)?,
            a: shift(*a, base)?,
            b: shift(*b, base)?,
            kind: *kind,
        },
        WirOp::FloatNeg { dst, src } => WirOp::FloatNeg {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
        },
        WirOp::DoubleNeg { dst, src } => WirOp::DoubleNeg {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
        },
        WirOp::CmpFloat { dst, a, b, bias } => WirOp::CmpFloat {
            dst: shift(*dst, base)?,
            a: shift(*a, base)?,
            b: shift(*b, base)?,
            bias: *bias,
        },
        WirOp::CmpDouble { dst, a, b, bias } => WirOp::CmpDouble {
            dst: shift(*dst, base)?,
            a: shift(*a, base)?,
            b: shift(*b, base)?,
            bias: *bias,
        },
        WirOp::Convert { dst, src, kind } => WirOp::Convert {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
            kind: *kind,
        },
        WirOp::FieldGet {
            dst,
            obj,
            slot,
            ty,
            field_index,
        } => WirOp::FieldGet {
            dst: shift(*dst, base)?,
            obj: shift(*obj, base)?,
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
            obj: shift(*obj, base)?,
            slot: *slot,
            src: shift(*src, base)?,
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
            args: args
                .iter()
                .map(|arg| shift(*arg, base))
                .collect::<Option<_>>()?,
        },
        WirOp::Move { dst, src, ty } => WirOp::Move {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
            ty: *ty,
        },
        WirOp::MoveResult { dst, ty } => WirOp::MoveResult {
            dst: shift(*dst, base)?,
            ty: *ty,
        },
        WirOp::Return { src, ty } => WirOp::Return {
            src: shift(*src, base)?,
            ty: *ty,
        },
        WirOp::ReturnVoid => WirOp::ReturnVoid,
        WirOp::IntNeg { dst, src } => WirOp::IntNeg {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
        },
        WirOp::IntNot { dst, src } => WirOp::IntNot {
            dst: shift(*dst, base)?,
            src: shift(*src, base)?,
        },
        // cite: art nodes.cc#InlineInto
        WirOp::ConstString { dst, string_index } => WirOp::ConstString {
            dst: shift(*dst, base)?,
            string_index: *string_index,
        },
        WirOp::HandlerFieldGet {
            dst,
            obj,
            field_index,
            ty,
        } => WirOp::HandlerFieldGet {
            dst: shift(*dst, base)?,
            obj: shift(*obj, base)?,
            field_index: *field_index,
            ty: *ty,
        },
        WirOp::HandlerFieldSet {
            obj,
            src,
            field_index,
            ty,
        } => WirOp::HandlerFieldSet {
            obj: shift(*obj, base)?,
            src: shift(*src, base)?,
            field_index: *field_index,
            ty: *ty,
        },
        WirOp::Goto { .. }
        | WirOp::IfCmp { .. }
        | WirOp::IfZero { .. }
        | WirOp::CheckCast { .. }
        | WirOp::InstanceOf { .. }
        | WirOp::Throw { .. }
        | WirOp::ConstClass { .. }
        | WirOp::NewInstance { .. }
        | WirOp::StaticGet { .. }
        | WirOp::StaticPut { .. }
        | WirOp::MoveException { .. }
        | WirOp::ArrayLength { .. }
        | WirOp::ArrayGet { .. }
        | WirOp::ArraySet { .. }
        | WirOp::NewArray { .. }
        | WirOp::FilledNewArray { .. }
        | WirOp::FillArrayData { .. }
        | WirOp::Switch { .. } => return None,
    })
}

pub fn inline_callee(
    callee: &InlinableCallee,
    arg_regs: &[u16],
    result: Option<(u16, RegType)>,
    reg_base: u16,
) -> Option<Vec<WirOp>> {
    if arg_regs.len() != callee.param_regs.len()
        || callee.param_types.len() != callee.param_regs.len()
    {
        return None;
    }

    let mut out = Vec::new();
    for ((param_reg, param_ty), arg_reg) in callee
        .param_regs
        .iter()
        .zip(callee.param_types.iter())
        .zip(arg_regs.iter())
    {
        out.push(WirOp::Move {
            dst: shift(*param_reg, reg_base)?,
            src: *arg_reg,
            ty: *param_ty,
        });
    }

    for op in &callee.ops {
        match op {
            WirOp::Return { src, ty } => {
                if let Some((dst, _)) = result {
                    out.push(WirOp::Move {
                        dst,
                        src: shift(*src, reg_base)?,
                        ty: *ty,
                    });
                }
            }
            WirOp::ReturnVoid => {}
            other => out.push(remap(other, reg_base)?),
        }
    }

    Some(out)
}

// cite: art inliner.cc#kMaximumNumberOfTotalInstructions
const MAX_TOTAL_INSTRUCTIONS: usize = 1024;

// cite: art inliner.cc#kMaximumNumberOfCumulatedDexRegisters
const MAX_CUMULATED_REGISTERS: u16 = 32;

// cite: art inliner.cc#TryInline, UpdateInliningBudget
pub fn inline_app_direct_calls(
    caller: &[WirOp],
    caller_register_count: u16,
    provider: &dyn CalleeProvider,
) -> Option<Vec<WirOp>> {
    let mut out = Vec::new();
    let mut next_base = caller_register_count;
    let mut inlined_instructions = 0usize;
    let mut cumulated_registers = 0u16;
    let mut index = 0;
    while index < caller.len() {
        if let WirOp::Invoke {
            kind: CallKind::AppDirect,
            method_index,
            args,
            ..
        } = &caller[index]
        {
            if let Some(callee) = provider.callee(*method_index) {
                // cite: art inliner.cc#UpdateInliningBudget
                let within_budget = inlined_instructions + callee.ops.len()
                    <= MAX_TOTAL_INSTRUCTIONS
                    && cumulated_registers.saturating_add(callee.register_count)
                        <= MAX_CUMULATED_REGISTERS;
                if within_budget {
                    let (result, consumed) = match caller.get(index + 1) {
                        Some(WirOp::MoveResult { dst, ty }) => (Some((*dst, *ty)), 2),
                        _ => (None, 1),
                    };
                    if let Some(body) = inline_callee(&callee, args, result, next_base) {
                        inlined_instructions += callee.ops.len();
                        cumulated_registers += callee.register_count;
                        out.extend(body);
                        next_base = next_base.checked_add(callee.register_count)?;
                        index += consumed;
                        continue;
                    }
                }
            }
        }
        out.push(caller[index].clone());
        index += 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex3oat::wir::IntBinKind;

    fn doubler() -> InlinableCallee {
        InlinableCallee {
            ops: vec![
                WirOp::IntBinOp {
                    dst: 0,
                    a: 1,
                    b: 1,
                    kind: IntBinKind::Add,
                },
                WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                },
            ],
            param_regs: vec![1],
            param_types: vec![RegType::Int32],
            register_count: 2,
        }
    }

    #[test]
    fn inlining_wires_params_renames_body_and_routes_the_return() {
        let body = inline_callee(&doubler(), &[3], Some((2, RegType::Int32)), 4).unwrap();
        assert_eq!(
            body,
            vec![
                WirOp::Move {
                    dst: 5,
                    src: 3,
                    ty: RegType::Int32,
                },
                WirOp::IntBinOp {
                    dst: 4,
                    a: 5,
                    b: 5,
                    kind: IntBinKind::Add,
                },
                WirOp::Move {
                    dst: 2,
                    src: 4,
                    ty: RegType::Int32,
                },
            ]
        );
    }

    #[test]
    fn a_discarded_return_drops_the_result_move() {
        let body = inline_callee(&doubler(), &[3], None, 4).unwrap();
        assert_eq!(body.len(), 2);
        assert!(matches!(body[1], WirOp::IntBinOp { dst: 4, .. }));
    }

    struct OneCallee;

    impl CalleeProvider for OneCallee {
        fn callee(&self, method_index: u32) -> Option<InlinableCallee> {
            (method_index == 7).then(doubler)
        }
    }

    #[test]
    fn the_driver_splices_an_app_direct_call_site_and_leaves_others() {
        let caller = vec![
            WirOp::Invoke {
                kind: CallKind::AppDirect,
                dex_kind: crate::dex3oat::wir::InvokeKind::Direct,
                method_index: 7,
                args: vec![0],
            },
            WirOp::MoveResult {
                dst: 1,
                ty: RegType::Int32,
            },
            WirOp::Invoke {
                kind: CallKind::FrameworkBoundary,
                dex_kind: crate::dex3oat::wir::InvokeKind::Virtual,
                method_index: 9,
                args: vec![1],
            },
            WirOp::Return {
                src: 1,
                ty: RegType::Int32,
            },
        ];
        let out = inline_app_direct_calls(&caller, 2, &OneCallee).unwrap();
        assert_eq!(
            out[0],
            WirOp::Move {
                dst: 3,
                src: 0,
                ty: RegType::Int32,
            }
        );
        assert_eq!(
            out[1],
            WirOp::IntBinOp {
                dst: 2,
                a: 3,
                b: 3,
                kind: IntBinKind::Add,
            }
        );
        assert_eq!(
            out[2],
            WirOp::Move {
                dst: 1,
                src: 2,
                ty: RegType::Int32,
            }
        );
        assert!(matches!(
            out[3],
            WirOp::Invoke {
                kind: CallKind::FrameworkBoundary,
                ..
            }
        ));
        assert_eq!(out.len(), 5);
    }

    struct WideCallee;

    impl CalleeProvider for WideCallee {
        fn callee(&self, method_index: u32) -> Option<InlinableCallee> {
            (method_index == 7).then(|| InlinableCallee {
                ops: vec![WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                }],
                param_regs: vec![0],
                param_types: vec![RegType::Int32],
                register_count: MAX_CUMULATED_REGISTERS + 1,
            })
        }
    }

    // cite: art inliner.cc#kMaximumNumberOfCumulatedDexRegisters
    #[test]
    fn a_callee_over_the_register_budget_is_left_as_a_call() {
        let caller = vec![
            WirOp::Invoke {
                kind: CallKind::AppDirect,
                dex_kind: crate::dex3oat::wir::InvokeKind::Direct,
                method_index: 7,
                args: vec![0],
            },
            WirOp::MoveResult {
                dst: 1,
                ty: RegType::Int32,
            },
            WirOp::Return {
                src: 1,
                ty: RegType::Int32,
            },
        ];
        let out = inline_app_direct_calls(&caller, 2, &WideCallee).unwrap();
        assert!(matches!(
            out[0],
            WirOp::Invoke {
                kind: CallKind::AppDirect,
                ..
            }
        ));
        assert_eq!(out.len(), 3);
    }
}

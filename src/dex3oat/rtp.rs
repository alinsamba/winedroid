use std::collections::HashMap;

use crate::dex3oat::graph::{BlockId, Graph};
use crate::dex3oat::ssa::reverse_post_order;
use crate::dex3oat::wir::WirOp;

// cite: art nodes.h#ReferenceTypeInfo, IsExact
pub type Facts = HashMap<u16, u32>;

// cite: art reference_type_propagation.cc#VisitNewInstance, VisitBasicBlock
fn transfer(op: &WirOp, facts: &mut Facts) {
    match op {
        WirOp::NewInstance { dst, type_index } => {
            facts.insert(*dst, *type_index);
        }
        WirOp::Move { dst, src, .. } => match facts.get(src).copied() {
            Some(ty) => {
                facts.insert(*dst, ty);
            }
            None => {
                facts.remove(dst);
            }
        },
        other => {
            if let Some(dst) = other.defined_register() {
                facts.remove(&dst);
            }
        }
    }
}

// cite: art reference_type_propagation.cc#MergeTypes
fn meet(into: &mut Facts, other: &Facts) {
    into.retain(|reg, ty| other.get(reg) == Some(ty));
}

// cite: art reference_type_propagation.cc#Run
pub fn exact_types(graph: &Graph) -> Vec<Facts> {
    let n = graph.blocks.len();
    let rpo = reverse_post_order(graph);
    let mut entry: Vec<Facts> = vec![HashMap::new(); n];
    let mut exit: Vec<Facts> = vec![HashMap::new(); n];
    let mut changed = true;
    while changed {
        changed = false;
        for &block_id in &rpo {
            let block = &graph.blocks[block_id];
            let mut incoming: Option<Facts> = None;
            for &pred in &block.predecessors {
                match &mut incoming {
                    None => incoming = Some(exit[pred].clone()),
                    Some(acc) => meet(acc, &exit[pred]),
                }
            }
            let incoming = incoming.unwrap_or_default();
            let mut facts = incoming.clone();
            for instr in &block.instructions {
                transfer(&instr.op, &mut facts);
            }
            if entry[block_id] != incoming || exit[block_id] != facts {
                entry[block_id] = incoming;
                exit[block_id] = facts;
                changed = true;
            }
        }
    }
    entry
}

// cite: art reference_type_propagation.cc#VisitInvoke
// cite: art nodes.h#IsExact
pub fn devirtualizable_receivers(graph: &Graph) -> Vec<(BlockId, usize, u32)> {
    let entry = exact_types(graph);
    let mut candidates = Vec::new();
    for block in &graph.blocks {
        let mut facts = entry[block.id].clone();
        for (index, instr) in block.instructions.iter().enumerate() {
            if let WirOp::Invoke { args, .. } = &instr.op {
                if let Some(receiver) = args.first() {
                    if let Some(ty) = facts.get(receiver).copied() {
                        candidates.push((block.id, index, ty));
                    }
                }
            }
            transfer(&instr.op, &mut facts);
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex3oat::graph::build;
    use crate::dex3oat::reg_type::RegType;
    use crate::dex3oat::wir::{CallKind, IfCmp};

    #[test]
    fn new_instance_gives_an_exact_receiver_at_the_call() {
        let wir = vec![
            (
                0,
                WirOp::NewInstance {
                    dst: 0,
                    type_index: 42,
                },
            ),
            (
                1,
                WirOp::Invoke {
                    kind: CallKind::FrameworkBoundary,
                    dex_kind: crate::dex3oat::wir::InvokeKind::Virtual,
                    method_index: 7,
                    args: vec![0],
                },
            ),
            (2, WirOp::ReturnVoid),
        ];
        let g = build(&wir, 1).expect("builds");
        let candidates = devirtualizable_receivers(&g);
        assert_eq!(candidates, vec![(0, 1, 42)]);
    }

    #[test]
    fn a_parameter_receiver_is_not_exact() {
        let wir = vec![
            (
                0,
                WirOp::Invoke {
                    kind: CallKind::FrameworkBoundary,
                    dex_kind: crate::dex3oat::wir::InvokeKind::Virtual,
                    method_index: 7,
                    args: vec![0],
                },
            ),
            (1, WirOp::ReturnVoid),
        ];
        let g = build(&wir, 1).expect("builds");
        assert!(devirtualizable_receivers(&g).is_empty());
    }

    #[test]
    fn types_that_differ_across_a_merge_are_not_exact() {
        let wir = vec![
            (
                0,
                WirOp::IfZero {
                    a: 1,
                    cmp: IfCmp::Ne,
                    target: 4,
                },
            ),
            (
                2,
                WirOp::NewInstance {
                    dst: 0,
                    type_index: 10,
                },
            ),
            (3, WirOp::Goto { target: 6 }),
            (
                4,
                WirOp::NewInstance {
                    dst: 0,
                    type_index: 20,
                },
            ),
            (
                6,
                WirOp::Invoke {
                    kind: CallKind::FrameworkBoundary,
                    dex_kind: crate::dex3oat::wir::InvokeKind::Virtual,
                    method_index: 7,
                    args: vec![0],
                },
            ),
            (7, WirOp::ReturnVoid),
        ];
        let g = build(&wir, 2).expect("builds");
        assert!(
            devirtualizable_receivers(&g).is_empty(),
            "v0 is type 10 or 20 depending on the branch -- not exact at the merge"
        );
    }

    #[test]
    fn same_type_on_both_arms_stays_exact() {
        let wir = vec![
            (
                0,
                WirOp::IfZero {
                    a: 1,
                    cmp: IfCmp::Ne,
                    target: 4,
                },
            ),
            (
                2,
                WirOp::NewInstance {
                    dst: 0,
                    type_index: 10,
                },
            ),
            (3, WirOp::Goto { target: 6 }),
            (
                4,
                WirOp::NewInstance {
                    dst: 0,
                    type_index: 10,
                },
            ),
            (
                6,
                WirOp::Invoke {
                    kind: CallKind::FrameworkBoundary,
                    dex_kind: crate::dex3oat::wir::InvokeKind::Virtual,
                    method_index: 7,
                    args: vec![0],
                },
            ),
            (
                7,
                WirOp::Return {
                    src: 0,
                    ty: RegType::Reference,
                },
            ),
        ];
        let g = build(&wir, 2).expect("builds");
        assert_eq!(devirtualizable_receivers(&g), vec![(3, 0, 10)]);
    }
}

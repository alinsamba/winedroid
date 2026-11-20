use std::collections::{HashMap, HashSet};

use crate::dex3oat::exec::{compare_taken, int_binop, ExecValue};
use crate::dex3oat::graph::{renumber, BasicBlock, BlockId, Graph};
use crate::dex3oat::wir::WirOp;

// cite: art constant_folding.cc#Run, InstructionWithAbsorbingInputSimplifier
pub fn constant_fold(graph: &mut Graph) {
    for block in &mut graph.blocks {
        let mut consts: HashMap<u16, i32> = HashMap::new();
        for instr in &mut block.instructions {
            match instr.op.clone() {
                WirOp::ConstInt { dst, value } => {
                    consts.insert(dst, value);
                }
                WirOp::IntBinOp { dst, a, b, kind } => {
                    let folded = match (consts.get(&a), consts.get(&b)) {
                        (Some(&x), Some(&y)) => int_binop(kind, x, y),
                        _ => None,
                    };
                    match folded {
                        Some(value) => {
                            instr.op = WirOp::ConstInt { dst, value };
                            consts.insert(dst, value);
                        }
                        None => {
                            consts.remove(&dst);
                        }
                    }
                }
                WirOp::IntBinLit {
                    dst,
                    src,
                    literal,
                    kind,
                } => {
                    let folded = consts.get(&src).and_then(|&x| int_binop(kind, x, literal));
                    match folded {
                        Some(value) => {
                            instr.op = WirOp::ConstInt { dst, value };
                            consts.insert(dst, value);
                        }
                        None => {
                            consts.remove(&dst);
                        }
                    }
                }
                WirOp::Move { dst, src, .. } => match consts.get(&src).copied() {
                    Some(value) => {
                        consts.insert(dst, value);
                    }
                    None => {
                        consts.remove(&dst);
                    }
                },
                other => {
                    if let Some(dst) = other.defined_register() {
                        consts.remove(&dst);
                    }
                }
            }
        }
        resolve_constant_branch(block, &consts);
    }
    renumber(graph);
    remove_unreachable_blocks(graph);
}

// cite: art dead_code_elimination.cc#SimplifyIfs
fn resolve_constant_branch(block: &mut BasicBlock, consts: &HashMap<u16, i32>) {
    if block.successors.len() != 2 {
        return;
    }
    let taken = match block.instructions.last().map(|i| &i.op) {
        Some(WirOp::IfZero { a, cmp, .. }) => consts
            .get(a)
            .and_then(|&v| compare_taken(*cmp, ExecValue::Int(v), ExecValue::Int(0))),
        Some(WirOp::IfCmp { a, b, cmp, .. }) => match (consts.get(a), consts.get(b)) {
            (Some(&x), Some(&y)) => compare_taken(*cmp, ExecValue::Int(x), ExecValue::Int(y)),
            _ => None,
        },
        _ => None,
    };
    if let Some(taken) = taken {
        let keep = if taken {
            block.successors[1]
        } else {
            block.successors[0]
        };
        if let Some(last) = block.instructions.last_mut() {
            last.op = WirOp::Goto { target: 0 };
        }
        block.successors = vec![keep];
    }
}

// cite: art nodes.cc#PopulateRecursive
fn natural_loop(graph: &Graph, header: BlockId, back_sources: &[BlockId]) -> HashSet<BlockId> {
    let mut blocks: HashSet<BlockId> = HashSet::from([header]);
    let mut stack: Vec<BlockId> = back_sources.to_vec();
    while let Some(block) = stack.pop() {
        if blocks.insert(block) {
            for pred in &graph.blocks[block].predecessors {
                stack.push(*pred);
            }
        }
    }
    blocks
}

// cite: art licm.cc#InputsAreDefinedBeforeLoop
fn hoist_loop_invariants(
    graph: &mut Graph,
    loop_blocks: &HashSet<BlockId>,
    pre_header: BlockId,
    rpo: &[BlockId],
) -> bool {
    let mut def_counts: HashMap<u16, u32> = HashMap::new();
    for block in loop_blocks {
        for instr in &graph.blocks[*block].instructions {
            if let Some(def) = instr.op.defined_register() {
                *def_counts.entry(def).or_insert(0) += 1;
            }
        }
    }
    let defined_in_loop: HashSet<u16> = def_counts.keys().copied().collect();

    let mut invariant: HashSet<u16> = HashSet::new();
    loop {
        let mut added = false;
        for block in loop_blocks {
            for instr in &graph.blocks[*block].instructions {
                let Some(def) = instr.op.defined_register() else {
                    continue;
                };
                if !instr.op.is_pure()
                    || invariant.contains(&def)
                    || def_counts.get(&def).copied().unwrap_or(0) != 1
                {
                    continue;
                }
                if instr
                    .op
                    .used_registers()
                    .iter()
                    .all(|r| !defined_in_loop.contains(r) || invariant.contains(r))
                {
                    invariant.insert(def);
                    added = true;
                }
            }
        }
        if !added {
            break;
        }
    }
    if invariant.is_empty() {
        return false;
    }

    let ordered: Vec<BlockId> = rpo
        .iter()
        .copied()
        .filter(|b| loop_blocks.contains(b))
        .collect();
    let mut hoisted_instrs = Vec::new();
    for block in ordered {
        let instrs = std::mem::take(&mut graph.blocks[block].instructions);
        let mut kept = Vec::with_capacity(instrs.len());
        for instr in instrs {
            if instr
                .op
                .defined_register()
                .is_some_and(|d| invariant.contains(&d))
            {
                hoisted_instrs.push(instr);
            } else {
                kept.push(instr);
            }
        }
        graph.blocks[block].instructions = kept;
    }
    let base = graph.blocks[pre_header]
        .instructions
        .len()
        .saturating_sub(1);
    for (offset, instr) in hoisted_instrs.into_iter().enumerate() {
        graph.blocks[pre_header]
            .instructions
            .insert(base + offset, instr);
    }
    true
}

// cite: art licm.cc#Run
pub fn loop_invariant_code_motion(graph: &mut Graph) {
    let rpo = crate::dex3oat::ssa::reverse_post_order(graph);
    let mut rpo_index = vec![usize::MAX; graph.blocks.len()];
    for (index, block) in rpo.iter().enumerate() {
        rpo_index[*block] = index;
    }
    // cite: art nodes.cc#UpdateDominatorOfSuccessor
    let mut loops: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for block in &graph.blocks {
        if rpo_index[block.id] == usize::MAX {
            continue;
        }
        for successor in &block.successors {
            if rpo_index[*successor] != usize::MAX && rpo_index[*successor] <= rpo_index[block.id] {
                loops.entry(*successor).or_default().push(block.id);
            }
        }
    }
    if loops.is_empty() {
        return;
    }

    let mut hoisted = false;
    for (header, sources) in &loops {
        let loop_blocks = natural_loop(graph, *header, sources);
        let pre: Vec<BlockId> = graph.blocks[*header]
            .predecessors
            .iter()
            .filter(|p| !loop_blocks.contains(p))
            .copied()
            .collect();
        if pre.len() != 1 || loop_blocks.contains(&pre[0]) {
            continue;
        }
        if hoist_loop_invariants(graph, &loop_blocks, pre[0], &rpo) {
            hoisted = true;
        }
    }
    if hoisted {
        renumber(graph);
    }
}

// cite: art gvn.cc#ValueSet, ComputeHashCode
fn value_key(op: &WirOp) -> Option<String> {
    match op {
        WirOp::ConstInt { value, .. } => Some(format!("ci:{value}")),
        WirOp::IntBinOp { a, b, kind, .. } => {
            let commutative = matches!(
                kind,
                crate::dex3oat::wir::IntBinKind::Add
                    | crate::dex3oat::wir::IntBinKind::Mul
                    | crate::dex3oat::wir::IntBinKind::And
                    | crate::dex3oat::wir::IntBinKind::Or
                    | crate::dex3oat::wir::IntBinKind::Xor
            );
            let (x, y) = if commutative && a > b { (b, a) } else { (a, b) };
            Some(format!("bo:{kind:?}:{x}:{y}"))
        }
        WirOp::IntBinLit {
            src, literal, kind, ..
        } => Some(format!("bl:{kind:?}:{src}:{literal}")),
        WirOp::IntNeg { src, .. } => Some(format!("neg:{src}")),
        WirOp::IntNot { src, .. } => Some(format!("not:{src}")),
        _ => None,
    }
}

// cite: art gvn.cc#Run, Lookup
pub fn global_value_numbering(graph: &mut Graph) {
    let mut changed = false;
    for block in &mut graph.blocks {
        let mut entries: Vec<(String, u16, Vec<u16>)> = Vec::new();
        for instr in &mut block.instructions {
            let key = value_key(&instr.op);
            let def = instr.op.defined_register();
            let inputs = instr.op.used_registers();
            let matched = key.as_ref().and_then(|key| {
                entries
                    .iter()
                    .find(|(k, _, _)| k == key)
                    .map(|(_, reg, _)| *reg)
            });
            if let (Some(def), Some(prev)) = (def, matched) {
                if prev != def {
                    instr.op = WirOp::Move {
                        dst: def,
                        src: prev,
                        ty: crate::dex3oat::reg_type::RegType::Int32,
                    };
                    changed = true;
                }
            }
            if let Some(def) = instr.op.defined_register() {
                entries.retain(|(_, res, ins)| *res != def && !ins.contains(&def));
            }
            if let (Some(key), Some(def)) = (key, def) {
                if matched.is_none() {
                    entries.push((key, def, inputs));
                }
            }
        }
    }
    if changed {
        renumber(graph);
    }
}

// cite: art ssa_liveness_analysis.cc#ComputeLiveRanges
// cite: art dead_code_elimination.cc#RemoveDeadInstructions
pub fn dead_code_elimination(graph: &mut Graph) {
    let n = graph.blocks.len();
    let mut live_in: Vec<HashSet<u16>> = vec![HashSet::new(); n];
    let mut changed = true;
    while changed {
        changed = false;
        for block in graph.blocks.iter().rev() {
            let mut live: HashSet<u16> = HashSet::new();
            for successor in &block.successors {
                live.extend(&live_in[*successor]);
            }
            for instr in block.instructions.iter().rev() {
                if let Some(def) = instr.op.defined_register() {
                    live.remove(&def);
                }
                for used in instr.op.used_registers() {
                    live.insert(used);
                }
            }
            if live != live_in[block.id] {
                live_in[block.id] = live;
                changed = true;
            }
        }
    }

    let live_out: Vec<HashSet<u16>> = graph
        .blocks
        .iter()
        .map(|block| {
            let mut out = HashSet::new();
            for successor in &block.successors {
                out.extend(&live_in[*successor]);
            }
            out
        })
        .collect();

    let mut removed_any = false;
    for (index, block) in graph.blocks.iter_mut().enumerate() {
        let mut live = live_out[index].clone();
        let mut keep = vec![true; block.instructions.len()];
        for (i, instr) in block.instructions.iter().enumerate().rev() {
            let dead = instr.op.is_pure()
                && instr
                    .op
                    .defined_register()
                    .is_some_and(|def| !live.contains(&def));
            if dead {
                keep[i] = false;
                removed_any = true;
                continue;
            }
            if let Some(def) = instr.op.defined_register() {
                live.remove(&def);
            }
            for used in instr.op.used_registers() {
                live.insert(used);
            }
        }
        let mut i = 0;
        block.instructions.retain(|_| {
            let k = keep[i];
            i += 1;
            k
        });
    }
    if removed_any {
        renumber(graph);
    }
}

// cite: art dead_code_elimination.cc#RemoveDeadBlocks
pub fn remove_unreachable_blocks(graph: &mut Graph) {
    let mut reachable = vec![false; graph.blocks.len()];
    let mut stack = vec![graph.entry];
    while let Some(block_id) = stack.pop() {
        if reachable.get(block_id).copied().unwrap_or(true) {
            continue;
        }
        reachable[block_id] = true;
        for successor in &graph.blocks[block_id].successors {
            stack.push(*successor);
        }
    }
    if reachable.iter().all(|r| *r) {
        return;
    }

    let mut new_id: Vec<Option<BlockId>> = vec![None; graph.blocks.len()];
    let mut next = 0;
    for (index, live) in reachable.iter().enumerate() {
        if *live {
            new_id[index] = Some(next);
            next += 1;
        }
    }

    let mut blocks = Vec::with_capacity(next);
    for (index, block) in graph.blocks.iter().enumerate() {
        if !reachable[index] {
            continue;
        }
        let mut kept = block.clone();
        kept.id = new_id[index].unwrap();
        kept.successors = block.successors.iter().filter_map(|s| new_id[*s]).collect();
        kept.predecessors = Vec::new();
        blocks.push(kept);
    }
    graph.entry = new_id[graph.entry].unwrap_or(0);
    graph.blocks = blocks;
    renumber(graph);
}

// cite: art dead_code_elimination.cc#SimplifyCFG
// cite: art nodes.cc#MergeWith
pub fn merge_blocks(graph: &mut Graph) {
    for _ in 0..graph.blocks.len().saturating_mul(2) + 1 {
        renumber(graph);
        let mut found: Option<(BlockId, BlockId)> = None;
        for block in &graph.blocks {
            if block.successors.len() != 1 {
                continue;
            }
            let successor = block.successors[0];
            if successor == block.id || successor == graph.entry {
                continue;
            }
            if graph.blocks[successor].predecessors == [block.id] {
                found = Some((block.id, successor));
                break;
            }
        }
        let Some((head, tail)) = found else {
            return;
        };
        let tail_instructions = graph.blocks[tail].instructions.clone();
        let tail_successors = graph.blocks[tail].successors.clone();
        {
            let head_block = &mut graph.blocks[head];
            if matches!(
                head_block.instructions.last().map(|i| &i.op),
                Some(WirOp::Goto { .. })
            ) {
                head_block.instructions.pop();
            }
            head_block.instructions.extend(tail_instructions);
            head_block.successors = tail_successors;
        }
        remove_unreachable_blocks(graph);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex3oat::exec::{ExecOutcome, ExecValue};
    use crate::dex3oat::graph::{build, execute};
    use crate::dex3oat::heap::Dex3oatHeap;
    use crate::dex3oat::reg_type::RegType;
    use crate::dex3oat::wir::{IfCmp, IntBinKind};

    fn run(graph: &Graph, start: &[ExecValue]) -> (ExecOutcome, Vec<ExecValue>) {
        let mut frame = start.to_vec();
        let outcome = execute(graph, &mut frame, &mut Dex3oatHeap::default());
        (outcome, frame)
    }

    #[test]
    fn merge_collapses_goto_chain_preserving_semantics() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 4 }),
            (1, WirOp::Goto { target: 2 }),
            (
                2,
                WirOp::IntBinLit {
                    dst: 0,
                    src: 0,
                    literal: 6,
                    kind: IntBinKind::Add,
                },
            ),
            (3, WirOp::Goto { target: 4 }),
            (
                4,
                WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                },
            ),
        ];
        let graph = build(&wir, 1).expect("builds");
        let before = run(&graph, &[ExecValue::Int(0)]);

        let mut merged = build(&wir, 1).expect("builds");
        let block_count_before = merged.blocks.len();
        merge_blocks(&mut merged);
        assert!(merged.blocks.len() < block_count_before, "chain collapsed");
        assert_eq!(run(&merged, &[ExecValue::Int(0)]), before);
        assert_eq!(before.0, ExecOutcome::Return(ExecValue::Int(10)));
    }

    #[test]
    fn constant_fold_folds_arithmetic_to_a_constant() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 2 }),
            (1, WirOp::ConstInt { dst: 1, value: 3 }),
            (
                2,
                WirOp::IntBinOp {
                    dst: 2,
                    a: 0,
                    b: 1,
                    kind: IntBinKind::Add,
                },
            ),
            (
                3,
                WirOp::IntBinLit {
                    dst: 2,
                    src: 2,
                    literal: 10,
                    kind: IntBinKind::Mul,
                },
            ),
            (
                4,
                WirOp::Return {
                    src: 2,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut g = build(&wir, 3).expect("builds");
        constant_fold(&mut g);
        let folded_constants = g
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .filter(|i| matches!(i.op, WirOp::ConstInt { dst: 2, .. }))
            .count();
        assert!(
            folded_constants >= 1,
            "the (2+3)*10 chain folded to a const"
        );
        assert_eq!(
            run(
                &g,
                &[ExecValue::Int(0), ExecValue::Int(0), ExecValue::Int(0)]
            )
            .0,
            ExecOutcome::Return(ExecValue::Int(50))
        );
    }

    #[test]
    fn constant_fold_resolves_a_constant_branch_and_kills_dead_arm() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 1 }),
            (
                1,
                WirOp::IfZero {
                    a: 0,
                    cmp: IfCmp::Ne,
                    target: 4,
                },
            ),
            (
                2,
                WirOp::Return {
                    src: 2,
                    ty: RegType::Int32,
                },
            ),
            (
                4,
                WirOp::Return {
                    src: 1,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut g = build(&wir, 3).expect("builds");
        let before = g.blocks.len();
        constant_fold(&mut g);
        assert!(g.blocks.len() < before, "dead arm removed");
        assert!(
            !g.blocks
                .iter()
                .flat_map(|b| &b.instructions)
                .any(|i| matches!(i.op, WirOp::IfZero { .. })),
            "constant branch resolved to a goto"
        );
        assert_eq!(
            run(
                &g,
                &[ExecValue::Int(1), ExecValue::Int(11), ExecValue::Int(22)]
            )
            .0,
            ExecOutcome::Return(ExecValue::Int(11))
        );
    }

    #[test]
    fn dce_removes_a_dead_pure_definition() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 5 }),
            (1, WirOp::ConstInt { dst: 1, value: 99 }),
            (
                2,
                WirOp::IntBinLit {
                    dst: 0,
                    src: 0,
                    literal: 10,
                    kind: IntBinKind::Add,
                },
            ),
            (
                3,
                WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut g = build(&wir, 2).expect("builds");
        let before = g.blocks.iter().map(|b| b.instructions.len()).sum::<usize>();
        dead_code_elimination(&mut g);
        let after = g.blocks.iter().map(|b| b.instructions.len()).sum::<usize>();
        assert_eq!(after, before - 1, "the dead const 99 was removed");
        assert!(
            !g.blocks
                .iter()
                .flat_map(|b| &b.instructions)
                .any(|i| matches!(i.op, WirOp::ConstInt { value: 99, .. })),
            "const 99 gone"
        );
        assert_eq!(
            run(&g, &[ExecValue::Int(0), ExecValue::Int(0)]).0,
            ExecOutcome::Return(ExecValue::Int(15))
        );
    }

    #[test]
    fn gvn_reuses_a_redundant_computation() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 5 }),
            (1, WirOp::ConstInt { dst: 1, value: 3 }),
            (
                2,
                WirOp::IntBinOp {
                    dst: 2,
                    a: 0,
                    b: 1,
                    kind: IntBinKind::Add,
                },
            ),
            (
                3,
                WirOp::IntBinOp {
                    dst: 3,
                    a: 1,
                    b: 0,
                    kind: IntBinKind::Add,
                },
            ),
            (
                4,
                WirOp::IntBinOp {
                    dst: 4,
                    a: 2,
                    b: 3,
                    kind: IntBinKind::Add,
                },
            ),
            (
                5,
                WirOp::Return {
                    src: 4,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut g = build(&wir, 5).expect("builds");
        global_value_numbering(&mut g);
        let add_count = g
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .filter(|i| matches!(i.op, WirOp::IntBinOp { .. }))
            .count();
        assert_eq!(
            add_count, 2,
            "the commutative-redundant v1+v0 became a Move"
        );
        assert_eq!(
            run(&g, &[ExecValue::Int(0); 5]).0,
            ExecOutcome::Return(ExecValue::Int(16))
        );
    }

    #[test]
    fn licm_hoists_a_loop_invariant() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 4, value: 0 }),
            (
                1,
                WirOp::IntBinOp {
                    dst: 3,
                    a: 1,
                    b: 2,
                    kind: IntBinKind::Add,
                },
            ),
            (
                2,
                WirOp::IntBinOp {
                    dst: 4,
                    a: 4,
                    b: 3,
                    kind: IntBinKind::Add,
                },
            ),
            (
                3,
                WirOp::IntBinLit {
                    dst: 0,
                    src: 0,
                    literal: -1,
                    kind: IntBinKind::Add,
                },
            ),
            (
                4,
                WirOp::IfZero {
                    a: 0,
                    cmp: IfCmp::Ne,
                    target: 1,
                },
            ),
            (
                5,
                WirOp::Return {
                    src: 4,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut g = build(&wir, 5).expect("builds");
        let loop_block = g
            .blocks
            .iter()
            .find(|b| b.predecessors.contains(&b.id))
            .expect("self-loop block")
            .id;
        loop_invariant_code_motion(&mut g);
        let invariant_in_loop = g.blocks[loop_block]
            .instructions
            .iter()
            .any(|i| matches!(i.op, WirOp::IntBinOp { a: 1, b: 2, .. }));
        assert!(!invariant_in_loop, "v1+v2 hoisted out of the loop");
        assert_eq!(
            run(
                &g,
                &[
                    ExecValue::Int(3),
                    ExecValue::Int(10),
                    ExecValue::Int(20),
                    ExecValue::Int(0),
                    ExecValue::Int(0)
                ]
            )
            .0,
            ExecOutcome::Return(ExecValue::Int(90))
        );
    }

    #[test]
    fn gvn_respects_redefinition() {
        let wir = vec![
            (
                0,
                WirOp::IntBinLit {
                    dst: 2,
                    src: 0,
                    literal: 1,
                    kind: IntBinKind::Add,
                },
            ),
            (1, WirOp::ConstInt { dst: 0, value: 100 }),
            (
                2,
                WirOp::IntBinLit {
                    dst: 3,
                    src: 0,
                    literal: 1,
                    kind: IntBinKind::Add,
                },
            ),
            (
                3,
                WirOp::IntBinOp {
                    dst: 4,
                    a: 2,
                    b: 3,
                    kind: IntBinKind::Add,
                },
            ),
            (
                4,
                WirOp::Return {
                    src: 4,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut g = build(&wir, 5).expect("builds");
        global_value_numbering(&mut g);
        assert_eq!(
            run(
                &g,
                &[
                    ExecValue::Int(10),
                    ExecValue::Int(0),
                    ExecValue::Int(0),
                    ExecValue::Int(0),
                    ExecValue::Int(0)
                ]
            )
            .0,
            ExecOutcome::Return(ExecValue::Int(11 + 101))
        );
    }

    #[test]
    fn dce_keeps_a_live_definition_and_side_effects() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 7 }),
            (
                1,
                WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut g = build(&wir, 1).expect("builds");
        dead_code_elimination(&mut g);
        assert_eq!(
            run(&g, &[ExecValue::Int(0)]).0,
            ExecOutcome::Return(ExecValue::Int(7))
        );
    }

    #[test]
    fn merge_leaves_branches_intact() {
        let wir = vec![
            (
                0,
                WirOp::IfZero {
                    a: 0,
                    cmp: IfCmp::Ne,
                    target: 3,
                },
            ),
            (
                1,
                WirOp::Return {
                    src: 2,
                    ty: RegType::Int32,
                },
            ),
            (
                3,
                WirOp::Return {
                    src: 1,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut merged = build(&wir, 3).expect("builds");
        merge_blocks(&mut merged);
        assert_eq!(
            run(
                &merged,
                &[ExecValue::Int(1), ExecValue::Int(11), ExecValue::Int(22)]
            )
            .0,
            ExecOutcome::Return(ExecValue::Int(11))
        );
        assert_eq!(
            run(
                &merged,
                &[ExecValue::Int(0), ExecValue::Int(11), ExecValue::Int(22)]
            )
            .0,
            ExecOutcome::Return(ExecValue::Int(22))
        );
    }
}

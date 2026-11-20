use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use crate::dex3oat::exec::{step_op, ExecOutcome, ExecValue, Step};
use crate::dex3oat::heap::Dex3oatHeap;
use crate::dex3oat::wir::WirOp;

pub type BlockId = usize;

// cite: art nodes.h#HEnvironment, GetMethod, SetAndCopyParentChain, GetParent
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub method_index: u32,
    pub declaring_class_descriptor: Rc<str>,
    pub dex_pc: usize,
    pub reg_base: u16,
    pub reg_count: u16,
    pub parent: Option<Box<Environment>>,
}

// cite: art nodes.h#HInstruction, GetDexPc, GetEnvironment
#[derive(Debug, Clone, PartialEq)]
pub struct GraphInstr {
    pub pc: usize,
    pub op: WirOp,
    pub env: Option<Environment>,
}

// cite: art nodes.h#HBasicBlock, GetPredecessors, GetSuccessors
#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub predecessors: Vec<BlockId>,
    pub successors: Vec<BlockId>,
    pub instructions: Vec<GraphInstr>,
}

// cite: art nodes.h#HGraph, GetEntryBlock, GetBlocks
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    pub register_count: u16,
}

const MAX_STEPS: usize = 100_000_000;

// cite: art nodes.h#IsControlFlow
fn is_terminator(op: &WirOp) -> bool {
    matches!(
        op,
        WirOp::Goto { .. }
            | WirOp::IfCmp { .. }
            | WirOp::IfZero { .. }
            | WirOp::Switch { .. }
            | WirOp::Return { .. }
            | WirOp::ReturnVoid
            | WirOp::Throw { .. }
    )
}

// cite: art block_builder.cc#CreateBranchTargets
fn explicit_targets(op: &WirOp) -> Vec<usize> {
    match op {
        WirOp::Goto { target } => vec![*target],
        WirOp::IfCmp { target, .. } | WirOp::IfZero { target, .. } => vec![*target],
        _ => Vec::new(),
    }
}

// cite: art block_builder.cc#CreateBranchTargets, MaybeCreateBlockAt
fn leaders(wir: &[(usize, WirOp)]) -> BTreeSet<usize> {
    let mut set = BTreeSet::new();
    if let Some((pc, _)) = wir.first() {
        set.insert(*pc);
    }
    for (index, (_, op)) in wir.iter().enumerate() {
        if is_terminator(op) {
            for target in explicit_targets(op) {
                set.insert(target);
            }
            if let Some((next_pc, _)) = wir.get(index + 1) {
                set.insert(*next_pc);
            }
        }
    }
    set
}

// cite: art block_builder.cc#Build, ConnectBasicBlocks
pub fn build(wir: &[(usize, WirOp)], register_count: u16) -> Option<Graph> {
    if wir.is_empty() {
        return None;
    }
    let leaders = leaders(wir);

    let mut blocks: Vec<BasicBlock> = Vec::new();
    let mut pc_to_block: HashMap<usize, BlockId> = HashMap::new();
    for (pc, op) in wir {
        if leaders.contains(pc) {
            let id = blocks.len();
            pc_to_block.insert(*pc, id);
            blocks.push(BasicBlock {
                id,
                predecessors: Vec::new(),
                successors: Vec::new(),
                instructions: Vec::new(),
            });
        }
        blocks.last_mut()?.instructions.push(GraphInstr {
            pc: *pc,
            op: op.clone(),
            env: None,
        });
    }

    let mut all_successors: Vec<Vec<BlockId>> = Vec::with_capacity(blocks.len());
    for index in 0..blocks.len() {
        let last = &blocks[index].instructions.last()?.op;
        all_successors.push(block_successors(last, index, blocks.len(), &pc_to_block)?);
    }
    for (index, successors) in all_successors.into_iter().enumerate() {
        for successor in &successors {
            blocks.get_mut(*successor)?.predecessors.push(index);
        }
        blocks[index].successors = successors;
    }

    Some(Graph {
        blocks,
        entry: 0,
        register_count,
    })
}

// cite: art block_builder.cc#ConnectBasicBlocks
fn block_successors(
    last: &WirOp,
    index: BlockId,
    block_count: usize,
    pc_to_block: &HashMap<usize, BlockId>,
) -> Option<Vec<BlockId>> {
    let fallthrough = index + 1;
    match last {
        WirOp::Return { .. } | WirOp::ReturnVoid | WirOp::Throw { .. } | WirOp::Switch { .. } => {
            Some(Vec::new())
        }
        WirOp::Goto { target } => Some(vec![*pc_to_block.get(target)?]),
        WirOp::IfCmp { target, .. } | WirOp::IfZero { target, .. } => {
            (fallthrough < block_count).then_some(vec![fallthrough, *pc_to_block.get(target)?])
        }
        _ => (fallthrough < block_count).then_some(vec![fallthrough]),
    }
}

// cite: art nodes.h#AddSuccessor
// cite: art block_builder.cc#ConnectBasicBlocks
pub fn renumber(graph: &mut Graph) {
    let mut pc = 0usize;
    let mut first_pc: Vec<usize> = vec![0; graph.blocks.len()];
    for (index, block) in graph.blocks.iter_mut().enumerate() {
        first_pc[index] = pc;
        for instr in &mut block.instructions {
            instr.pc = pc;
            pc += 1;
        }
    }
    for block in &mut graph.blocks {
        let successors = block.successors.clone();
        let Some(last) = block.instructions.last_mut() else {
            continue;
        };
        let retargeted = match &last.op {
            WirOp::Goto { .. } => successors.first().map(|s| WirOp::Goto {
                target: first_pc[*s],
            }),
            WirOp::IfCmp { .. } | WirOp::IfZero { .. } => successors
                .get(1)
                .map(|s| last.op.retarget(|_| first_pc[*s])),
            _ => None,
        };
        if let Some(op) = retargeted {
            last.op = op;
        }
    }
    let mut predecessors: Vec<Vec<BlockId>> = vec![Vec::new(); graph.blocks.len()];
    for block in &graph.blocks {
        for successor in &block.successors {
            predecessors[*successor].push(block.id);
        }
    }
    for (index, block) in graph.blocks.iter_mut().enumerate() {
        block.predecessors = std::mem::take(&mut predecessors[index]);
    }
}

// cite: art linear_order.h#LinearizeGraph
pub fn flatten(graph: &Graph) -> Vec<(usize, WirOp)> {
    let mut out = Vec::new();
    for block in &graph.blocks {
        for instr in &block.instructions {
            out.push((instr.pc, instr.op.clone()));
        }
    }
    out
}

// cite: art code_generator.cc#RecordPcInfo
// cite: art stack_map.h#StackMap
pub fn deopt_stacks(graph: &Graph) -> Vec<(usize, Vec<crate::dex3oat::deopt::ResumePoint>)> {
    let mut out = Vec::new();
    for block in &graph.blocks {
        for instr in &block.instructions {
            let stack = match &instr.env {
                Some(env) => crate::dex3oat::deopt::deopt_frames(env),
                None => vec![crate::dex3oat::deopt::ResumePoint {
                    method_index: 0,
                    declaring_class_descriptor: None,
                    dex_pc: instr.pc,
                    reg_base: 0,
                    reg_count: graph.register_count,
                }],
            };
            out.push((instr.pc, stack));
        }
    }
    out
}

// cite: art nodes.h#HGraph
// cite: art code_generator.cc#GenerateSlowPaths
pub fn execute(graph: &Graph, frame: &mut [ExecValue], heap: &mut Dex3oatHeap) -> ExecOutcome {
    let mut identity: HashMap<usize, usize> = HashMap::new();
    let mut pc_to_block: HashMap<usize, BlockId> = HashMap::new();
    for block in &graph.blocks {
        if let Some(first) = block.instructions.first() {
            pc_to_block.insert(first.pc, block.id);
        }
        for instr in &block.instructions {
            identity.insert(instr.pc, instr.pc);
        }
    }

    let mut block_id = graph.entry;
    for _ in 0..MAX_STEPS {
        let Some(block) = graph.blocks.get(block_id) else {
            return ExecOutcome::Error;
        };
        let mut jumped = false;
        for instr in &block.instructions {
            match step_op(&instr.op, frame, heap, &identity) {
                Some(Step::Next) => {}
                Some(Step::Jump(target_pc)) => match pc_to_block.get(&target_pc) {
                    Some(id) => {
                        block_id = *id;
                        jumped = true;
                        break;
                    }
                    None => return ExecOutcome::Error,
                },
                Some(Step::Finish(outcome)) => return outcome,
                None => return ExecOutcome::Error,
            }
        }
        if !jumped {
            match block.successors.first() {
                Some(id) => block_id = *id,
                None => return ExecOutcome::Error,
            }
        }
    }
    ExecOutcome::Error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex3oat::exec::execute as flat_execute;
    use crate::dex3oat::wir::IfCmp;

    fn heap() -> Dex3oatHeap {
        Dex3oatHeap::default()
    }

    fn assert_same(wir: &[(usize, WirOp)], start: &[ExecValue]) {
        let graph = build(wir, start.len() as u16).expect("graph builds");

        let mut flat_frame = start.to_vec();
        let flat = flat_execute(wir, &mut flat_frame, &mut heap());

        let mut graph_frame = start.to_vec();
        let via_graph = execute(&graph, &mut graph_frame, &mut heap());

        assert_eq!(flat, via_graph, "outcome mismatch for {wir:?}");
        assert_eq!(flat_frame, graph_frame, "frame mismatch for {wir:?}");

        let mut renumbered = build(wir, start.len() as u16).expect("graph builds");
        renumber(&mut renumbered);
        let mut renum_frame = start.to_vec();
        let via_renum = execute(&renumbered, &mut renum_frame, &mut heap());
        assert_eq!(flat, via_renum, "renumber outcome mismatch for {wir:?}");
        assert_eq!(
            flat_frame, renum_frame,
            "renumber frame mismatch for {wir:?}"
        );

        let flattened = flatten(&renumbered);
        let mut flat2_frame = start.to_vec();
        let via_flat2 = flat_execute(&flattened, &mut flat2_frame, &mut heap());
        assert_eq!(flat, via_flat2, "flatten outcome mismatch for {wir:?}");
        assert_eq!(
            flat_frame, flat2_frame,
            "flatten frame mismatch for {wir:?}"
        );
    }

    #[test]
    fn straight_line_matches_flat() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 7 }),
            (
                1,
                WirOp::IntBinOp {
                    dst: 0,
                    a: 0,
                    b: 1,
                    kind: crate::dex3oat::wir::IntBinKind::Add,
                },
            ),
            (
                2,
                WirOp::Return {
                    src: 0,
                    ty: crate::dex3oat::reg_type::RegType::Int32,
                },
            ),
        ];
        assert_same(&wir, &[ExecValue::Int(0), ExecValue::Int(5)]);
    }

    #[test]
    fn if_branch_taken_and_not_taken_match_flat() {
        let wir = vec![
            (
                0,
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
                    ty: crate::dex3oat::reg_type::RegType::Int32,
                },
            ),
            (
                4,
                WirOp::Return {
                    src: 1,
                    ty: crate::dex3oat::reg_type::RegType::Int32,
                },
            ),
        ];
        assert_same(
            &wir,
            &[ExecValue::Int(1), ExecValue::Int(11), ExecValue::Int(22)],
        );
        assert_same(
            &wir,
            &[ExecValue::Int(0), ExecValue::Int(11), ExecValue::Int(22)],
        );
    }

    #[test]
    fn loop_with_back_edge_matches_flat() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 1, value: 0 }),
            (
                1,
                WirOp::IntBinOp {
                    dst: 1,
                    a: 1,
                    b: 2,
                    kind: crate::dex3oat::wir::IntBinKind::Add,
                },
            ),
            (
                2,
                WirOp::IntBinLit {
                    dst: 0,
                    src: 0,
                    literal: -1,
                    kind: crate::dex3oat::wir::IntBinKind::Add,
                },
            ),
            (
                3,
                WirOp::IfZero {
                    a: 0,
                    cmp: IfCmp::Ne,
                    target: 1,
                },
            ),
            (
                4,
                WirOp::Return {
                    src: 1,
                    ty: crate::dex3oat::reg_type::RegType::Int32,
                },
            ),
        ];
        assert_same(
            &wir,
            &[ExecValue::Int(3), ExecValue::Int(0), ExecValue::Int(10)],
        );
    }

    #[test]
    fn deopt_op_matches_flat() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 1 }),
            (
                1,
                WirOp::ConstString {
                    dst: 1,
                    string_index: 0,
                },
            ),
            (
                2,
                WirOp::Return {
                    src: 0,
                    ty: crate::dex3oat::reg_type::RegType::Int32,
                },
            ),
        ];
        assert_same(&wir, &[ExecValue::Int(0), ExecValue::Int(0)]);
    }
}

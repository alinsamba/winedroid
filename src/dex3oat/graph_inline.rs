use std::rc::Rc;

use crate::dex3oat::graph::{renumber, BasicBlock, BlockId, Environment, Graph, GraphInstr};
use crate::dex3oat::reg_type::RegType;
use crate::dex3oat::wir::{CallKind, WirOp};

// cite: art inliner.cc#TryBuildAndInline
pub struct GraphCallee {
    pub graph: Graph,
    pub method_index: u32,
    pub declaring_class_descriptor: Rc<str>,
    pub param_regs: Vec<u16>,
    pub param_types: Vec<RegType>,
}

fn graph_size(graph: &Graph) -> usize {
    graph
        .blocks
        .iter()
        .map(|block| block.instructions.len())
        .sum()
}

// cite: art inliner.cc#TryInline
// cite: art nodes.h#GetParent
fn on_inlining_path(env: Option<&Environment>, method_index: u32) -> bool {
    let mut current = env;
    while let Some(node) = current {
        if node.method_index == method_index {
            return true;
        }
        current = node.parent.as_deref();
    }
    false
}

// cite: art inliner.cc#Run, UpdateInliningBudget
pub fn run_inliner(
    caller: &mut Graph,
    resolve: &dyn Fn(u32) -> Option<GraphCallee>,
    instruction_budget: usize,
    register_ceiling: u16,
) -> usize {
    let mut inlined_instructions = 0usize;
    loop {
        let mut target: Option<(BlockId, usize, GraphCallee)> = None;
        'scan: for block in &caller.blocks {
            for (index, instr) in block.instructions.iter().enumerate() {
                let WirOp::Invoke {
                    kind: CallKind::AppDirect,
                    method_index,
                    ..
                } = &instr.op
                else {
                    continue;
                };
                if on_inlining_path(instr.env.as_ref(), *method_index) {
                    continue;
                }
                let Some(callee) = resolve(*method_index) else {
                    continue;
                };
                if inlined_instructions + graph_size(&callee.graph) > instruction_budget {
                    continue;
                }
                // cite: art inliner.cc#kMaximumNumberOfCumulatedDexRegisters
                if caller
                    .register_count
                    .saturating_add(callee.graph.register_count)
                    > register_ceiling
                {
                    continue;
                }
                target = Some((block.id, index, callee));
                break 'scan;
            }
        }
        let Some((block_id, index, callee)) = target else {
            return inlined_instructions;
        };
        let size = graph_size(&callee.graph);
        if inline_call(
            caller,
            block_id,
            index,
            &callee.graph,
            callee.method_index,
            &callee.declaring_class_descriptor,
            &callee.param_regs,
            &callee.param_types,
        )
        .is_none()
        {
            return inlined_instructions;
        }
        inlined_instructions += size;
    }
}

// cite: art nodes.cc#InlineInto
// cite: art inliner.cc#TryBuildAndInlineHelper
pub fn inline_call(
    caller: &mut Graph,
    block_id: BlockId,
    instr_index: usize,
    callee: &Graph,
    callee_method_index: u32,
    callee_declaring_class_descriptor: &Rc<str>,
    callee_param_regs: &[u16],
    callee_param_types: &[RegType],
) -> Option<()> {
    let block = caller.blocks.get(block_id)?;
    let invoke = block.instructions.get(instr_index)?;
    let args = match &invoke.op {
        WirOp::Invoke { args, .. } => args.clone(),
        _ => return None,
    };
    if args.len() != callee_param_regs.len() || args.len() != callee_param_types.len() {
        return None;
    }
    let invoke_env = invoke.env.clone();
    let (result, consumed) = match block.instructions.get(instr_index + 1) {
        Some(GraphInstr {
            op: WirOp::MoveResult { dst, ty },
            ..
        }) => (Some((*dst, *ty)), 2),
        _ => (None, 1),
    };
    if instr_index + consumed >= block.instructions.len() {
        return None;
    }

    let reg_base = caller.register_count;

    let before: Vec<GraphInstr> = block.instructions[..instr_index].to_vec();
    let after: Vec<GraphInstr> = block.instructions[instr_index + consumed..].to_vec();
    let orig_successors = block.successors.clone();

    let cont_id = caller.blocks.len();
    let callee_base = cont_id + 1;

    caller.blocks.push(BasicBlock {
        id: cont_id,
        predecessors: Vec::new(),
        successors: orig_successors,
        instructions: after,
    });

    let mut head = before;
    for ((param, arg), ty) in callee_param_regs.iter().zip(&args).zip(callee_param_types) {
        head.push(GraphInstr {
            pc: 0,
            op: WirOp::Move {
                dst: param + reg_base,
                src: *arg,
                ty: *ty,
            },
            env: invoke_env.clone(),
        });
    }
    head.push(GraphInstr {
        pc: 0,
        op: WirOp::Goto { target: 0 },
        env: invoke_env.clone(),
    });
    caller.blocks[block_id].instructions = head;
    caller.blocks[block_id].successors = vec![callee_base + callee.entry];

    for callee_block in &callee.blocks {
        let new_id = callee_base + callee_block.id;
        let mut instructions = Vec::with_capacity(callee_block.instructions.len());
        let mut successors: Vec<BlockId> = callee_block
            .successors
            .iter()
            .map(|s| callee_base + s)
            .collect();
        for instr in &callee_block.instructions {
            let env = Some(Environment {
                method_index: callee_method_index,
                declaring_class_descriptor: Rc::clone(callee_declaring_class_descriptor),
                dex_pc: instr.pc,
                reg_base,
                reg_count: callee.register_count,
                parent: invoke_env.clone().map(Box::new),
            });
            match &instr.op {
                WirOp::Return { src, ty } => {
                    if let Some((dst, _)) = result {
                        instructions.push(GraphInstr {
                            pc: 0,
                            op: WirOp::Move {
                                dst,
                                src: src + reg_base,
                                ty: *ty,
                            },
                            env: env.clone(),
                        });
                    }
                    instructions.push(GraphInstr {
                        pc: 0,
                        op: WirOp::Goto { target: 0 },
                        env,
                    });
                    successors = vec![cont_id];
                }
                WirOp::ReturnVoid => {
                    instructions.push(GraphInstr {
                        pc: 0,
                        op: WirOp::Goto { target: 0 },
                        env,
                    });
                    successors = vec![cont_id];
                }
                other => instructions.push(GraphInstr {
                    pc: 0,
                    op: other.map_registers(|r| r + reg_base),
                    env,
                }),
            }
        }
        caller.blocks.push(BasicBlock {
            id: new_id,
            predecessors: Vec::new(),
            successors,
            instructions,
        });
    }

    caller.register_count = reg_base + callee.register_count;
    renumber(caller);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex3oat::deopt::annotate_environments;
    use crate::dex3oat::exec::{ExecOutcome, ExecValue};
    use crate::dex3oat::graph::{build, execute};
    use crate::dex3oat::heap::Dex3oatHeap;
    use crate::dex3oat::wir::{CallKind, IfCmp};

    fn declaring_class(method_index: u32) -> Rc<str> {
        Rc::from(format!("Ltest/Method{method_index};"))
    }

    fn int(v: i32) -> ExecValue {
        ExecValue::Int(v)
    }

    fn max_callee() -> Graph {
        let wir = vec![
            (
                0,
                WirOp::IfCmp {
                    a: 0,
                    b: 1,
                    cmp: IfCmp::Gt,
                    target: 3,
                },
            ),
            (
                1,
                WirOp::Return {
                    src: 1,
                    ty: RegType::Int32,
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
        let mut g = build(&wir, 2).expect("callee builds");
        annotate_environments(&mut g, 99, declaring_class(99));
        g
    }

    fn caller_calling_max(v0: i32, v1: i32) -> Graph {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: v0 }),
            (1, WirOp::ConstInt { dst: 1, value: v1 }),
            (
                2,
                WirOp::Invoke {
                    kind: CallKind::AppDirect,
                    dex_kind: crate::dex3oat::wir::InvokeKind::Direct,
                    method_index: 99,
                    args: vec![0, 1],
                },
            ),
            (
                3,
                WirOp::MoveResult {
                    dst: 2,
                    ty: RegType::Int32,
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
        let mut g = build(&wir, 3).expect("caller builds");
        annotate_environments(&mut g, 1, declaring_class(1));
        g
    }

    fn run_inlined(v0: i32, v1: i32) -> ExecOutcome {
        let callee = max_callee();
        let mut caller = caller_calling_max(v0, v1);
        inline_call(
            &mut caller,
            0,
            2,
            &callee,
            99,
            &declaring_class(99),
            &[0, 1],
            &[RegType::Int32, RegType::Int32],
        )
        .expect("inline succeeds");
        let mut frame = vec![int(0); caller.register_count as usize];
        execute(&caller, &mut frame, &mut Dex3oatHeap::default())
    }

    #[test]
    fn inlined_branchy_callee_first_branch() {
        assert_eq!(run_inlined(5, 3), ExecOutcome::Return(int(5)));
    }

    #[test]
    fn inlined_branchy_callee_second_branch() {
        assert_eq!(run_inlined(3, 5), ExecOutcome::Return(int(5)));
    }

    fn find_invoke(g: &Graph) -> Option<(BlockId, usize)> {
        for block in &g.blocks {
            for (index, instr) in block.instructions.iter().enumerate() {
                if matches!(instr.op, WirOp::Invoke { .. }) {
                    return Some((block.id, index));
                }
            }
        }
        None
    }

    fn double_callee() -> Graph {
        let wir = vec![
            (
                0,
                WirOp::IntBinOp {
                    dst: 0,
                    a: 0,
                    b: 0,
                    kind: crate::dex3oat::wir::IntBinKind::Add,
                },
            ),
            (
                1,
                WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                },
            ),
        ];
        let mut g = build(&wir, 1).expect("double builds");
        annotate_environments(&mut g, 3, declaring_class(3));
        g
    }

    #[test]
    fn driver_inlines_a_resolved_call() {
        let mut top = {
            let wir = vec![
                (0, WirOp::ConstInt { dst: 0, value: 9 }),
                (
                    1,
                    WirOp::Invoke {
                        kind: CallKind::AppDirect,
                        dex_kind: crate::dex3oat::wir::InvokeKind::Direct,
                        method_index: 3,
                        args: vec![0],
                    },
                ),
                (
                    2,
                    WirOp::MoveResult {
                        dst: 1,
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
            let mut g = build(&wir, 2).expect("top builds");
            annotate_environments(&mut g, 1, declaring_class(1));
            g
        };
        let resolve = |method_index: u32| -> Option<GraphCallee> {
            (method_index == 3).then(|| GraphCallee {
                graph: double_callee(),
                method_index: 3,
                declaring_class_descriptor: declaring_class(3),
                param_regs: vec![0],
                param_types: vec![RegType::Int32],
            })
        };
        let inlined = run_inliner(&mut top, &resolve, 64, 64);
        assert!(inlined > 0, "the call was inlined");
        assert!(find_invoke(&top).is_none(), "no invokes remain");
        let mut frame = vec![int(0); top.register_count as usize];
        assert_eq!(
            execute(&top, &mut frame, &mut Dex3oatHeap::default()),
            ExecOutcome::Return(int(18))
        );
    }

    #[test]
    fn driver_guards_direct_recursion() {
        let recursive = || {
            let wir = vec![
                (
                    0,
                    WirOp::Invoke {
                        kind: CallKind::AppDirect,
                        dex_kind: crate::dex3oat::wir::InvokeKind::Direct,
                        method_index: 5,
                        args: vec![0],
                    },
                ),
                (
                    1,
                    WirOp::MoveResult {
                        dst: 0,
                        ty: RegType::Int32,
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
            let mut g = build(&wir, 1).expect("recursive builds");
            annotate_environments(&mut g, 5, declaring_class(5));
            g
        };
        let mut top = recursive();
        let resolve = |method_index: u32| -> Option<GraphCallee> {
            (method_index == 5).then(|| GraphCallee {
                graph: recursive(),
                method_index: 5,
                declaring_class_descriptor: declaring_class(5),
                param_regs: vec![0],
                param_types: vec![RegType::Int32],
            })
        };
        let inlined = run_inliner(&mut top, &resolve, 1024, 128);
        assert!(inlined <= 1024, "recursion guard + budget terminate");
    }

    #[test]
    fn recursive_inline_double_then_triple_chains_three_deep() {
        let double = {
            let wir = vec![
                (
                    0,
                    WirOp::IntBinOp {
                        dst: 0,
                        a: 0,
                        b: 0,
                        kind: crate::dex3oat::wir::IntBinKind::Add,
                    },
                ),
                (
                    1,
                    WirOp::Return {
                        src: 0,
                        ty: RegType::Int32,
                    },
                ),
            ];
            let mut g = build(&wir, 1).expect("double builds");
            annotate_environments(&mut g, 3, declaring_class(3));
            g
        };
        let triple = {
            let wir = vec![
                (
                    0,
                    WirOp::Invoke {
                        kind: CallKind::AppDirect,
                        dex_kind: crate::dex3oat::wir::InvokeKind::Direct,
                        method_index: 3,
                        args: vec![0],
                    },
                ),
                (
                    1,
                    WirOp::MoveResult {
                        dst: 1,
                        ty: RegType::Int32,
                    },
                ),
                (
                    2,
                    WirOp::IntBinOp {
                        dst: 1,
                        a: 1,
                        b: 0,
                        kind: crate::dex3oat::wir::IntBinKind::Add,
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
            let mut g = build(&wir, 2).expect("triple builds");
            annotate_environments(&mut g, 2, declaring_class(2));
            g
        };
        let mut top = {
            let wir = vec![
                (0, WirOp::ConstInt { dst: 0, value: 7 }),
                (
                    1,
                    WirOp::Invoke {
                        kind: CallKind::AppDirect,
                        dex_kind: crate::dex3oat::wir::InvokeKind::Direct,
                        method_index: 2,
                        args: vec![0],
                    },
                ),
                (
                    2,
                    WirOp::MoveResult {
                        dst: 1,
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
            let mut g = build(&wir, 2).expect("top builds");
            annotate_environments(&mut g, 1, declaring_class(1));
            g
        };

        let (b, i) = find_invoke(&top).expect("top has the call to triple");
        inline_call(
            &mut top,
            b,
            i,
            &triple,
            2,
            &declaring_class(2),
            &[0],
            &[RegType::Int32],
        )
        .expect("inline triple");
        let (b, i) = find_invoke(&top).expect("inlined triple still calls double");
        inline_call(
            &mut top,
            b,
            i,
            &double,
            3,
            &declaring_class(3),
            &[0],
            &[RegType::Int32],
        )
        .expect("inline double");

        assert!(find_invoke(&top).is_none(), "all calls inlined");
        let mut frame = vec![int(0); top.register_count as usize];
        let outcome = execute(&top, &mut frame, &mut Dex3oatHeap::default());
        assert_eq!(outcome, ExecOutcome::Return(int(21)));

        let deepest = top
            .blocks
            .iter()
            .flat_map(|blk| &blk.instructions)
            .filter_map(|instr| instr.env.as_ref())
            .map(|env| crate::dex3oat::deopt::deopt_frames(env).len())
            .max()
            .unwrap_or(0);
        assert_eq!(
            deepest, 3,
            "deepest inlined instruction reconstructs 3 frames"
        );
    }

    // cite: art nodes.h#HEnvironment, GetMethod, SetAndCopyParentChain
    #[test]
    fn inline_chains_the_environment_parent_to_the_call_site() {
        let callee = max_callee();
        let mut caller = caller_calling_max(5, 3);
        let resolved_body: Rc<str> = Rc::from("Ltest/ResolvedOverride;");
        inline_call(
            &mut caller,
            0,
            2,
            &callee,
            99,
            &resolved_body,
            &[0, 1],
            &[RegType::Int32, RegType::Int32],
        )
        .expect("inline succeeds");
        let mut inlined_env_seen = false;
        for block in &caller.blocks {
            for instr in &block.instructions {
                if let Some(env) = &instr.env {
                    if env.method_index == 99 {
                        inlined_env_seen = true;
                        assert_eq!(env.declaring_class_descriptor, resolved_body);
                        let resume_frames = crate::dex3oat::deopt::deopt_frames(env);
                        assert_eq!(
                            resume_frames
                                .last()
                                .and_then(|frame| frame.declaring_class_descriptor.as_deref()),
                            Some("Ltest/ResolvedOverride;")
                        );
                        let parent = env.parent.as_ref().expect("callee env has a parent");
                        assert_eq!(parent.method_index, 1);
                        assert_eq!(env.reg_base, 3);
                    }
                }
            }
        }
        assert!(inlined_env_seen, "callee instructions were spliced in");
    }
}

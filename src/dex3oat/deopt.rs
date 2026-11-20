use std::rc::Rc;

use crate::dex3oat::graph::{Environment, Graph};

// cite: art stack_map.h#StackMap, InlineInfo
// cite: art code_generator.cc#RecordPcInfo, EmitEnvironment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumePoint {
    pub method_index: u32,
    pub declaring_class_descriptor: Option<Rc<str>>,
    pub dex_pc: usize,
    pub reg_base: u16,
    pub reg_count: u16,
}

impl From<&Environment> for ResumePoint {
    fn from(env: &Environment) -> Self {
        ResumePoint {
            method_index: env.method_index,
            declaring_class_descriptor: Some(Rc::clone(&env.declaring_class_descriptor)),
            dex_pc: env.dex_pc,
            reg_base: env.reg_base,
            reg_count: env.reg_count,
        }
    }
}

// cite: art nodes.h#SetRawEnvironment, GetMethod
// cite: art code_generator.cc#RecordPcInfo
pub fn annotate_environments(
    graph: &mut Graph,
    method_index: u32,
    declaring_class_descriptor: Rc<str>,
) {
    let reg_count = graph.register_count;
    for block in &mut graph.blocks {
        for instr in &mut block.instructions {
            instr.env = Some(Environment {
                method_index,
                declaring_class_descriptor: Rc::clone(&declaring_class_descriptor),
                dex_pc: instr.pc,
                reg_base: 0,
                reg_count,
                parent: None,
            });
        }
    }
}

// cite: art quick_exception_handler.cc#DeoptimizeStackVisitor
// cite: art nodes.h#GetParent
pub fn outermost_resume(env: &Environment) -> ResumePoint {
    let mut current = env;
    while let Some(parent) = &current.parent {
        current = parent;
    }
    ResumePoint::from(current)
}

// cite: art quick_exception_handler.cc#DeoptimizeStackVisitor
// cite: art stack_map.h#DexRegisterMap
pub fn resume_window<T: Clone>(point: &ResumePoint, frame: &[T]) -> Option<Vec<T>> {
    let start = point.reg_base as usize;
    let end = start.checked_add(point.reg_count as usize)?;
    frame.get(start..end).map(<[T]>::to_vec)
}

// cite: art quick_exception_handler.cc#DeoptimizeStackVisitor
// cite: art nodes.h#IsFromInlinedInvoke
pub fn deopt_frames(env: &Environment) -> Vec<ResumePoint> {
    let mut frames = Vec::new();
    let mut current = Some(env);
    while let Some(node) = current {
        frames.push(ResumePoint::from(node));
        current = node.parent.as_deref();
    }
    frames.reverse();
    frames
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex3oat::graph::{build, Environment};
    use crate::dex3oat::wir::WirOp;

    fn env(method_index: u32, dex_pc: usize, reg_base: u16, reg_count: u16) -> Environment {
        Environment {
            method_index,
            declaring_class_descriptor: Rc::from(format!("Ltest/Method{method_index};")),
            dex_pc,
            reg_base,
            reg_count,
            parent: None,
        }
    }

    #[test]
    fn annotate_sets_top_level_env_on_every_instruction() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 1 }),
            (
                1,
                WirOp::Return {
                    src: 0,
                    ty: crate::dex3oat::reg_type::RegType::Int32,
                },
            ),
        ];
        let mut graph = build(&wir, 3).expect("graph builds");
        annotate_environments(&mut graph, 42, Rc::from("Ltest/Body;"));
        for block in &graph.blocks {
            for instr in &block.instructions {
                let env = instr.env.as_ref().expect("env populated");
                assert_eq!(env.method_index, 42);
                assert_eq!(&*env.declaring_class_descriptor, "Ltest/Body;");
                assert_eq!(env.dex_pc, instr.pc);
                assert_eq!(env.reg_base, 0);
                assert_eq!(env.reg_count, 3);
                assert!(env.parent.is_none());
            }
        }
    }

    #[test]
    fn flat_env_resumes_itself() {
        let e = env(7, 12, 0, 5);
        assert_eq!(
            outermost_resume(&e),
            ResumePoint {
                method_index: 7,
                declaring_class_descriptor: Some(Rc::from("Ltest/Method7;")),
                dex_pc: 12,
                reg_base: 0,
                reg_count: 5,
            }
        );
        assert_eq!(deopt_frames(&e), vec![ResumePoint::from(&e)]);
    }

    #[test]
    fn nested_env_resumes_at_outermost_call_site() {
        let mut inner = env(3, 5, 20, 4);
        inner.parent = Some(Box::new(env(1, 8, 0, 10)));
        assert_eq!(
            outermost_resume(&inner),
            ResumePoint {
                method_index: 1,
                declaring_class_descriptor: Some(Rc::from("Ltest/Method1;")),
                dex_pc: 8,
                reg_base: 0,
                reg_count: 10,
            }
        );
    }

    // cite: art nodes.h#HEnvironment, GetMethod, GetParent
    #[test]
    fn deopt_frames_reconstruct_the_full_stack_outermost_first() {
        let mut c = env(3, 2, 30, 3);
        let mut b = env(2, 6, 15, 6);
        b.parent = Some(Box::new(env(1, 9, 0, 12)));
        c.parent = Some(Box::new(b));
        let frames = deopt_frames(&c);
        assert_eq!(
            frames,
            vec![
                ResumePoint {
                    method_index: 1,
                    declaring_class_descriptor: Some(Rc::from("Ltest/Method1;")),
                    dex_pc: 9,
                    reg_base: 0,
                    reg_count: 12,
                },
                ResumePoint {
                    method_index: 2,
                    declaring_class_descriptor: Some(Rc::from("Ltest/Method2;")),
                    dex_pc: 6,
                    reg_base: 15,
                    reg_count: 6,
                },
                ResumePoint {
                    method_index: 3,
                    declaring_class_descriptor: Some(Rc::from("Ltest/Method3;")),
                    dex_pc: 2,
                    reg_base: 30,
                    reg_count: 3,
                },
            ]
        );
        assert_eq!(frames.first().unwrap(), &outermost_resume(&c));
    }

    #[test]
    fn resume_window_extracts_the_frame_slice() {
        let frame = vec![10, 11, 12, 13, 14, 15];
        let top = ResumePoint {
            method_index: 1,
            declaring_class_descriptor: Some(Rc::from("Ltest/Method1;")),
            dex_pc: 4,
            reg_base: 0,
            reg_count: 3,
        };
        assert_eq!(resume_window(&top, &frame), Some(vec![10, 11, 12]));
        let inlined = ResumePoint {
            method_index: 2,
            declaring_class_descriptor: Some(Rc::from("Ltest/Method2;")),
            dex_pc: 0,
            reg_base: 3,
            reg_count: 3,
        };
        assert_eq!(resume_window(&inlined, &frame), Some(vec![13, 14, 15]));
        let out_of_range = ResumePoint {
            method_index: 3,
            declaring_class_descriptor: Some(Rc::from("Ltest/Method3;")),
            dex_pc: 0,
            reg_base: 4,
            reg_count: 4,
        };
        assert_eq!(resume_window::<i32>(&out_of_range, &frame), None);
    }
}

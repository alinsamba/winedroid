use std::cell::RefCell;
use std::rc::Rc;

use super::vm::ResolvedMethod;
use super::{DexProgram, DEX_ACC_STATIC};
use crate::dex::DexCodeItem;
use crate::dex3oat::exec::{execute, ExecOutcome, ExecValue};
use crate::dex3oat::heap::Dex3oatHeap;
use crate::dex3oat::inline::{inline_callee, InlinableCallee};
use crate::dex3oat::linker::{
    ClassSource, Dex3oatLinker, LinkableClass, LinkableField, LinkableMethod,
};
use crate::dex3oat::reg_type::RegType;
use crate::dex3oat::verifier::{verify_method, ReturnType, VerifierTypes};
use crate::dex3oat::wir::{lower_method, CallKind, WirOp, WirResolver};
use crate::dex_interpreter::{
    interpret_code_with_initial_arguments_in_context, DexExecutionResult, DexInvocation,
    DexInvocationHandler, DexInvocationResult, DexValue,
};
use crate::error::{Result, WinedroidError};

// cite: art inliner.cc#kMaximumNumberOfCumulatedDexRegisters
pub(super) const INLINE_REGISTER_BUDGET: u16 = 32;

struct CachedCallee {
    wir: Vec<(usize, WirOp)>,
    param_types: Vec<RegType>,
    register_count: u16,
    ins_size: u16,
}

thread_local! {
    static CALLEE_CACHE: RefCell<std::collections::HashMap<(usize, u32), Option<std::rc::Rc<CachedCallee>>>> =
        RefCell::new(std::collections::HashMap::new());
}

// cite: art reg_type_cache.cc#FromDescriptor
fn reg_type_of_descriptor(type_descriptor: &str) -> RegType {
    match type_descriptor.as_bytes().first() {
        Some(b'J') => RegType::LongLo,
        Some(b'D') => RegType::DoubleLo,
        Some(b'F') => RegType::Float,
        Some(b'L' | b'[') => RegType::Reference,
        Some(b'Z' | b'B' | b'C' | b'S' | b'I') => RegType::Int32,
        _ => RegType::Conflict,
    }
}

pub(super) struct Dex3oatBridge<'a> {
    program: &'a DexProgram,
    dex_file_index: usize,
    linker: RefCell<Dex3oatLinker>,
}

impl<'a> Dex3oatBridge<'a> {
    pub(super) fn new(program: &'a DexProgram, dex_file_index: usize) -> Self {
        Self {
            program,
            dex_file_index,
            linker: RefCell::new(Dex3oatLinker::new()),
        }
    }

    // cite: art modifiers.h#kAccStatic
    fn entry_parameter_types(
        &self,
        resolved: &ResolvedMethod,
        code: &DexCodeItem,
    ) -> Option<Vec<RegType>> {
        let mut types = Vec::new();
        if resolved.encoded_method.access_flags & DEX_ACC_STATIC == 0 {
            types.push(RegType::Reference);
        }
        for parameter in &resolved.method.proto.parameters {
            match reg_type_of_descriptor(parameter) {
                RegType::LongLo => {
                    types.push(RegType::LongLo);
                    types.push(RegType::LongHi);
                }
                RegType::DoubleLo => {
                    types.push(RegType::DoubleLo);
                    types.push(RegType::DoubleHi);
                }
                other => types.push(other),
            }
        }
        (types.len() == code.ins_size as usize).then_some(types)
    }

    // cite: art inliner.cc#TryInline
    fn inlinable_callee(&self, method_index: u32) -> Option<InlinableCallee> {
        let id = self
            .program
            .method_id(self.dex_file_index, method_index)
            .ok()?;
        let resolved = self
            .program
            .resolve_exact_method_in_class_hierarchy(&id.class_descriptor, &id)
            .ok()??;
        if resolved.dex_file_index != self.dex_file_index
            || resolved.encoded_method.code_offset == 0
        {
            return None;
        }
        let code = self.program.code_item(&resolved).ok()?;
        let parameter_types = self.entry_parameter_types(&resolved, &code)?;
        let verified = verify_method(
            &code.instructions,
            code.registers_size,
            &parameter_types,
            &code.try_blocks,
            self,
        )?;
        let wir = lower_method(&code.instructions, &verified, self)?;
        // cite: art inliner.cc#kMaximumNumberOfInstructionsForSmallMethod
        if wir.len() > 8 {
            return None;
        }
        for (_, op) in &wir {
            if matches!(
                op,
                WirOp::Invoke { .. }
                    | WirOp::Goto { .. }
                    | WirOp::IfCmp { .. }
                    | WirOp::IfZero { .. }
                    | WirOp::Switch { .. }
                    | WirOp::Throw { .. }
                    | WirOp::MoveException { .. }
            ) {
                return None;
            }
        }
        let register_count = code.registers_size;
        let first = register_count.checked_sub(code.ins_size)?;
        let param_regs: Vec<u16> = (first..register_count).collect();
        let ops: Vec<WirOp> = wir.into_iter().map(|(_, op)| op).collect();
        Some(InlinableCallee {
            ops,
            param_regs,
            param_types: parameter_types,
            register_count,
        })
    }

    fn float_intrinsic_method_sets(
        &self,
        wir: &[(usize, WirOp)],
    ) -> (
        std::collections::HashSet<u32>,
        std::collections::HashSet<u32>,
    ) {
        let mut to_bits = std::collections::HashSet::new();
        let mut from_bits = std::collections::HashSet::new();
        for (_, op) in wir {
            if let WirOp::Invoke { method_index, .. } = op {
                if let Ok(id) = self.program.method_id(self.dex_file_index, *method_index) {
                    if id.class_descriptor == "Ljava/lang/Float;" {
                        match id.name.as_str() {
                            "floatToRawIntBits" | "floatToIntBits" => {
                                to_bits.insert(*method_index);
                            }
                            "intBitsToFloat" => {
                                from_bits.insert(*method_index);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        (to_bits, from_bits)
    }

    // cite: art optimizing_compiler.cc#TryCompile
    // cite: art inliner.cc#TryInline
    pub(super) fn object_ops(
        &self,
        resolved: &ResolvedMethod,
        code: &DexCodeItem,
    ) -> Option<Vec<crate::jit::ObjectOp>> {
        let parameter_types = self.entry_parameter_types(resolved, code)?;
        let verified = verify_method(
            &code.instructions,
            code.registers_size,
            &parameter_types,
            &code.try_blocks,
            self,
        )?;
        let wir = lower_method(&code.instructions, &verified, self)?;

        // cite: art inliner.cc#kMaximumNumberOfCumulatedDexRegisters
        let register_budget = code.registers_size.saturating_add(INLINE_REGISTER_BUDGET);
        let mut fused: Vec<(usize, WirOp)> = Vec::with_capacity(wir.len());
        let mut next_base = code.registers_size;
        let mut index = 0;
        while index < wir.len() {
            let (pc, op) = &wir[index];
            if let WirOp::Invoke {
                kind: CallKind::AppDirect,
                method_index,
                args,
                ..
            } = op
            {
                if let Some(callee) = self.inlinable_callee(*method_index) {
                    let fits = next_base
                        .checked_add(callee.register_count)
                        .is_some_and(|total| total <= register_budget);
                    if fits {
                        let (result, consumed) = match wir.get(index + 1) {
                            Some((_, WirOp::MoveResult { dst, ty })) => (Some((*dst, *ty)), 2),
                            _ => (None, 1),
                        };
                        if let Some(body) = inline_callee(&callee, args, result, next_base) {
                            fused.extend(body.into_iter().map(|op| (*pc, op)));
                            next_base += callee.register_count;
                            index += consumed;
                            continue;
                        }
                    }
                }
            }
            fused.push((*pc, op.clone()));
            index += 1;
        }

        let (to_bits, from_bits) = self.float_intrinsic_method_sets(&fused);
        crate::dex3oat::codegen::wir_to_object_ops(&fused, &to_bits, &from_bits)
    }

    // cite: art inliner.cc#TryBuildAndInline
    fn inlinable_graph_callee(
        &self,
        method_index: u32,
    ) -> Option<crate::dex3oat::graph_inline::GraphCallee> {
        let id = self
            .program
            .method_id(self.dex_file_index, method_index)
            .ok()?;
        self.inlinable_graph_callee_in(method_index, &id.class_descriptor)
    }

    // cite: art inliner.cc#TryInlineMonomorphicCall
    fn inlinable_graph_callee_in(
        &self,
        method_index: u32,
        class_descriptor: &str,
    ) -> Option<crate::dex3oat::graph_inline::GraphCallee> {
        let id = self
            .program
            .method_id(self.dex_file_index, method_index)
            .ok()?;
        let resolved = self
            .program
            .resolve_exact_method_in_class_hierarchy(class_descriptor, &id)
            .ok()??;
        if resolved.dex_file_index != self.dex_file_index
            || resolved.encoded_method.code_offset == 0
        {
            return None;
        }
        let key = (self.dex_file_index, resolved.encoded_method.code_offset);
        let cached = match CALLEE_CACHE.with(|c| c.borrow().get(&key).cloned()) {
            Some(entry) => entry,
            None => {
                let computed = self.compute_cached_callee(&resolved);
                CALLEE_CACHE.with(|c| c.borrow_mut().insert(key, computed.clone()));
                computed
            }
        }?;

        let mut graph = crate::dex3oat::graph::build(&cached.wir, cached.register_count)?;
        let declaring_class_descriptor = Rc::from(resolved.class_descriptor.as_str());
        crate::dex3oat::deopt::annotate_environments(
            &mut graph,
            method_index,
            Rc::clone(&declaring_class_descriptor),
        );
        let first = cached.register_count.checked_sub(cached.ins_size)?;
        Some(crate::dex3oat::graph_inline::GraphCallee {
            graph,
            method_index,
            declaring_class_descriptor,
            param_regs: (first..cached.register_count).collect(),
            param_types: cached.param_types.clone(),
        })
    }

    // cite: art inliner.cc#TryBuildAndInline, TryInlineFromCHA
    fn compute_cached_callee(
        &self,
        resolved: &ResolvedMethod,
    ) -> Option<std::rc::Rc<CachedCallee>> {
        let code = self.program.code_item(resolved).ok()?;
        let parameter_types = self.entry_parameter_types(resolved, &code)?;
        if !code.try_blocks.is_empty() {
            return None;
        }
        let verified = verify_method(
            &code.instructions,
            code.registers_size,
            &parameter_types,
            &code.try_blocks,
            self,
        )?;
        let wir = lower_method(&code.instructions, &verified, self)?;
        if wir.len() > 16 {
            return None;
        }
        for (_, op) in &wir {
            if matches!(
                op,
                WirOp::Switch { .. } | WirOp::Throw { .. } | WirOp::MoveException { .. }
            ) {
                return None;
            }
        }
        Some(std::rc::Rc::new(CachedCallee {
            wir,
            param_types: parameter_types,
            register_count: code.registers_size,
            ins_size: code.ins_size,
        }))
    }

    // cite: art inliner.cc#TryInlineMonomorphicCall
    // cite: art reference_type_propagation.cc#VisitInvoke
    fn run_devirt_inline(&self, graph: &mut crate::dex3oat::graph::Graph, register_ceiling: u16) {
        for _ in 0..8 {
            let mut inlined = false;
            for (block, index, type_index) in crate::dex3oat::rtp::devirtualizable_receivers(graph)
            {
                let Some(instr) = graph
                    .blocks
                    .get(block)
                    .and_then(|b| b.instructions.get(index))
                else {
                    continue;
                };
                let WirOp::Invoke { method_index, .. } = &instr.op else {
                    continue;
                };
                let method_index = *method_index;
                let Ok(descriptor) = self
                    .program
                    .type_descriptor(self.dex_file_index, type_index)
                else {
                    continue;
                };
                let Some(callee) = self.inlinable_graph_callee_in(method_index, &descriptor) else {
                    continue;
                };
                if graph
                    .register_count
                    .saturating_add(callee.graph.register_count)
                    > register_ceiling
                {
                    continue;
                }
                if crate::dex3oat::graph_inline::inline_call(
                    graph,
                    block,
                    index,
                    &callee.graph,
                    callee.method_index,
                    &callee.declaring_class_descriptor,
                    &callee.param_regs,
                    &callee.param_types,
                )
                .is_some()
                {
                    inlined = true;
                    break;
                }
            }
            if !inlined {
                return;
            }
        }
    }

    // cite: art optimizing_compiler.cc#RunOptimizations
    // cite: art code_generator.cc#Compile
    pub(super) fn object_ops_graph(
        &self,
        resolved: &ResolvedMethod,
        code: &DexCodeItem,
    ) -> Option<(Vec<crate::jit::ObjectOp>, crate::jit::CompiledMeta)> {
        if std::env::var_os("WD_JIT_COMPILE_TRACE").is_some() {
            let result = self.object_ops_graph_trace_result(resolved, code);
            match &result {
                Ok(_) => eprintln!(
                    "JITGRAPH OK   {}->{}{}",
                    resolved.method.class_descriptor,
                    resolved.method.name,
                    resolved.method.proto.descriptor()
                ),
                Err(reason) => eprintln!(
                    "JITGRAPH FAIL {}->{}{} {reason}",
                    resolved.method.class_descriptor,
                    resolved.method.name,
                    resolved.method.proto.descriptor()
                ),
            }
            return result.ok();
        }
        self.object_ops_graph_inner(resolved, code)
    }

    fn object_ops_graph_inner(
        &self,
        resolved: &ResolvedMethod,
        code: &DexCodeItem,
    ) -> Option<(Vec<crate::jit::ObjectOp>, crate::jit::CompiledMeta)> {
        let parameter_types = self.entry_parameter_types(resolved, code)?;
        let verified = verify_method(
            &code.instructions,
            code.registers_size,
            &parameter_types,
            &code.try_blocks,
            self,
        )?;
        let wir = lower_method(&code.instructions, &verified, self)?;

        let mut graph = crate::dex3oat::graph::build(&wir, code.registers_size)?;
        crate::dex3oat::deopt::annotate_environments(
            &mut graph,
            0,
            Rc::from(resolved.class_descriptor.as_str()),
        );
        if std::env::var_os("WINEDROID_DEX3OAT_INLINE").is_some() {
            let ceiling = code.registers_size.saturating_add(INLINE_REGISTER_BUDGET);
            let resolve = |method_index: u32| self.inlinable_graph_callee(method_index);
            crate::dex3oat::graph_inline::run_inliner(&mut graph, &resolve, 48, ceiling);
            self.run_devirt_inline(&mut graph, ceiling);
        }
        crate::dex3oat::graph_opt::constant_fold(&mut graph);
        crate::dex3oat::graph_opt::global_value_numbering(&mut graph);
        crate::dex3oat::graph_opt::dead_code_elimination(&mut graph);

        let flat = crate::dex3oat::graph::flatten(&graph);
        let (to_bits, from_bits) = self.float_intrinsic_method_sets(&flat);
        let ops = crate::dex3oat::codegen::wir_to_object_ops(&flat, &to_bits, &from_bits)?;
        let meta = crate::jit::CompiledMeta {
            deopt: crate::dex3oat::graph::deopt_stacks(&graph)
                .into_iter()
                .map(|(codegen, stack)| (codegen as u32, stack))
                .collect(),
        };
        Some((ops, meta))
    }

    fn object_ops_graph_trace_result(
        &self,
        resolved: &ResolvedMethod,
        code: &DexCodeItem,
    ) -> std::result::Result<(Vec<crate::jit::ObjectOp>, crate::jit::CompiledMeta), String> {
        let parameter_types = self
            .entry_parameter_types(resolved, code)
            .ok_or_else(|| "reject-params".to_string())?;
        let verified = verify_method(
            &code.instructions,
            code.registers_size,
            &parameter_types,
            &code.try_blocks,
            self,
        )
        .ok_or_else(|| "verify-bail".to_string())?;
        let wir = lower_method(&code.instructions, &verified, self)
            .ok_or_else(|| "lower-bail".to_string())?;

        let mut graph = crate::dex3oat::graph::build(&wir, code.registers_size)
            .ok_or_else(|| "graph-build-bail".to_string())?;
        crate::dex3oat::deopt::annotate_environments(
            &mut graph,
            0,
            Rc::from(resolved.class_descriptor.as_str()),
        );
        if std::env::var_os("WINEDROID_DEX3OAT_INLINE").is_some() {
            let ceiling = code.registers_size.saturating_add(INLINE_REGISTER_BUDGET);
            let resolve = |method_index: u32| self.inlinable_graph_callee(method_index);
            crate::dex3oat::graph_inline::run_inliner(&mut graph, &resolve, 48, ceiling);
            self.run_devirt_inline(&mut graph, ceiling);
        }
        crate::dex3oat::graph_opt::constant_fold(&mut graph);
        crate::dex3oat::graph_opt::global_value_numbering(&mut graph);
        crate::dex3oat::graph_opt::dead_code_elimination(&mut graph);

        let flat = crate::dex3oat::graph::flatten(&graph);
        let (to_bits, from_bits) = self.float_intrinsic_method_sets(&flat);
        let ops = crate::dex3oat::codegen::wir_to_object_ops(&flat, &to_bits, &from_bits)
            .ok_or_else(|| {
                let reason = crate::dex3oat::codegen::object_codegen_reject_reason(
                    &flat, &to_bits, &from_bits,
                )
                .unwrap_or_else(|| "unknown".to_string());
                format!("codegen-bail {reason}")
            })?;
        let meta = crate::jit::CompiledMeta {
            deopt: crate::dex3oat::graph::deopt_stacks(&graph)
                .into_iter()
                .map(|(codegen, stack)| (codegen as u32, stack))
                .collect(),
        };
        Ok((ops, meta))
    }

    fn coverage_outcome(&self, resolved: &ResolvedMethod, code: &DexCodeItem) -> &'static str {
        let Some(parameter_types) = self.entry_parameter_types(resolved, code) else {
            return "reject-params";
        };
        let Some(verified) = verify_method(
            &code.instructions,
            code.registers_size,
            &parameter_types,
            &code.try_blocks,
            self,
        ) else {
            return "verify-bail";
        };
        match lower_method(&code.instructions, &verified, self) {
            Some(_) => "lowered",
            None => "lower-bail",
        }
    }

    // cite: art interpreter.cc#Execute
    fn live_differential(
        &self,
        resolved: &ResolvedMethod,
        code: &DexCodeItem,
        arguments: &[DexValue],
    ) {
        if !arguments
            .iter()
            .all(|a| matches!(a, DexValue::Int(_) | DexValue::Long(_) | DexValue::Float(_)))
        {
            return;
        }
        let Some(parameter_types) = self.entry_parameter_types(resolved, code) else {
            return;
        };
        let Some(verified) = verify_method(
            &code.instructions,
            code.registers_size,
            &parameter_types,
            &code.try_blocks,
            self,
        ) else {
            return;
        };
        let Some(wir) = lower_method(&code.instructions, &verified, self) else {
            return;
        };

        let mut frame = vec![ExecValue::Uninitialized; code.registers_size as usize];
        let first_param = code.registers_size as usize - arguments.len();
        for (offset, value) in arguments.iter().enumerate() {
            frame[first_param + offset] = match value {
                DexValue::Int(v) => ExecValue::Int(*v),
                DexValue::Long(v) => ExecValue::Long(*v),
                DexValue::Float(v) => ExecValue::Float(*v),
                _ => ExecValue::Reference(0),
            };
        }

        let mut heap = Dex3oatHeap::new();
        let mut graph_frame = frame.clone();
        let outcome = execute(&wir, &mut frame, &mut heap);
        if let Some(graph) = crate::dex3oat::graph::build(&wir, code.registers_size) {
            let mut graph_heap = Dex3oatHeap::new();
            let graph_outcome =
                crate::dex3oat::graph::execute(&graph, &mut graph_frame, &mut graph_heap);
            if graph_outcome != outcome || graph_frame != frame {
                crate::trace::event(
                    "dex3oat",
                    format!(
                        "GRAPH-MISMATCH flat={outcome:?} graph={graph_outcome:?} {}",
                        resolved.method.display_signature()
                    ),
                );
            }
        }
        let executed = match outcome {
            ExecOutcome::Return(value @ (ExecValue::Int(_) | ExecValue::Long(_))) => value,
            ExecOutcome::Return(value @ (ExecValue::Float(_) | ExecValue::Double(_))) => value,
            ExecOutcome::Error => {
                crate::trace::event(
                    "dex3oat",
                    format!("EXECUTOR-ERROR {}", resolved.method.display_signature()),
                );
                return;
            }
            _ => return,
        };

        let mut handler = DeoptOnCall;
        let interpreted = interpret_code_with_initial_arguments_in_context(
            code,
            Some(arguments),
            &mut handler,
            0,
        );
        let Ok(interpreted) = interpreted else {
            return;
        };
        if !exec_matches_result(executed, &interpreted) {
            crate::trace::event(
                "dex3oat",
                format!(
                    "MISMATCH exec={executed:?} interp={interpreted:?} {}",
                    resolved.method.display_signature()
                ),
            );
        }
    }
}

// cite: art interpreter_common.h#DoCall
struct DeoptOnCall;

impl DexInvocationHandler for DeoptOnCall {
    fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
        Err(WinedroidError::new("dex3oat live-diff reached a call"))
    }
}

// cite: art interpreter.cc#Execute
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

impl ClassSource for Dex3oatBridge<'_> {
    fn linkable_class(&self, descriptor: &str) -> Option<LinkableClass> {
        let loaded = self.program.load_class(descriptor)?;
        let dex = loaded.dex_file_index;
        let mut instance_fields = Vec::new();
        for field in &loaded.class_def.instance_fields {
            let id = self.program.field_id(dex, field.field_index).ok()?;
            instance_fields.push(LinkableField {
                name: id.name.clone(),
                type_descriptor: id.type_descriptor.clone(),
            });
        }
        let mut static_fields = Vec::new();
        for field in &loaded.class_def.static_fields {
            let id = self.program.field_id(dex, field.field_index).ok()?;
            static_fields.push(LinkableField {
                name: id.name.clone(),
                type_descriptor: id.type_descriptor.clone(),
            });
        }
        let mut virtual_methods = Vec::new();
        for method in &loaded.class_def.virtual_methods {
            let id = self.program.method_id(dex, method.method_index).ok()?;
            virtual_methods.push(LinkableMethod {
                name: id.name.clone(),
                proto: id.proto.descriptor(),
            });
        }
        Some(LinkableClass {
            descriptor: descriptor.to_owned(),
            superclass: loaded.class_def.superclass_descriptor.clone(),
            instance_fields,
            static_fields,
            virtual_methods,
        })
    }
}

impl VerifierTypes for Dex3oatBridge<'_> {
    fn field_type(&self, field_index: u32) -> Option<RegType> {
        let id = self
            .program
            .field_id(self.dex_file_index, field_index)
            .ok()?;
        Some(reg_type_of_descriptor(&id.type_descriptor))
    }

    fn method_return_type(&self, method_index: u32) -> Option<ReturnType> {
        let id = self
            .program
            .method_id(self.dex_file_index, method_index)
            .ok()?;
        Some(if id.proto.return_type == "V" {
            ReturnType::Void
        } else {
            ReturnType::Value(reg_type_of_descriptor(&id.proto.return_type))
        })
    }
}

impl WirResolver for Dex3oatBridge<'_> {
    // cite: art reg_type_cache.cc#FromDescriptor
    fn field_type(&self, field_index: u32) -> Option<RegType> {
        let id = self
            .program
            .field_id(self.dex_file_index, field_index)
            .ok()?;
        Some(reg_type_of_descriptor(&id.type_descriptor))
    }

    // cite: art class_linker.cc#LinkFields
    // cite: art art_field.h#GetOffset
    fn field_slot_and_type(&self, field_index: u32) -> Option<(u16, RegType)> {
        let id = self
            .program
            .field_id(self.dex_file_index, field_index)
            .ok()?;
        let ty = reg_type_of_descriptor(&id.type_descriptor);
        let mut linker = self.linker.borrow_mut();
        let layout = linker.link_class(self, &id.class_descriptor)?;
        let slot = layout.slot_of(&id.name, &id.type_descriptor)?;
        Some((slot, ty))
    }

    // cite: art reg_type_cache.cc#FromDescriptor
    // cite: art class_linker.cc#CreateArrayClass
    fn array_component(&self, type_index: u32) -> Option<RegType> {
        let descriptor = self
            .program
            .type_descriptor(self.dex_file_index, type_index)
            .ok()?;
        let element = descriptor.strip_prefix('[')?;
        Some(reg_type_of_descriptor(element))
    }

    // cite: art class_linker.cc#ResolveMethod
    fn call_kind(&self, method_index: u32) -> Option<CallKind> {
        let id = self
            .program
            .method_id(self.dex_file_index, method_index)
            .ok()?;
        match self
            .program
            .resolve_exact_method_in_class_hierarchy(&id.class_descriptor, &id)
        {
            Ok(Some(resolved)) if resolved.encoded_method.code_offset != 0 => {
                Some(CallKind::AppDirect)
            }
            _ => Some(CallKind::FrameworkBoundary),
        }
    }
}

thread_local! {
    static PROBED: RefCell<std::collections::HashSet<(usize, u32)>> =
        RefCell::new(std::collections::HashSet::new());
}

pub(super) fn probe_coverage(
    program: &DexProgram,
    resolved: &ResolvedMethod,
    code: &DexCodeItem,
    arguments: Option<&[DexValue]>,
) {
    let bridge = Dex3oatBridge::new(program, resolved.dex_file_index);
    if let Some(arguments) = arguments {
        bridge.live_differential(resolved, code, arguments);
    }

    let key = (resolved.dex_file_index, resolved.encoded_method.code_offset);
    let fresh = PROBED.with(|probed| probed.borrow_mut().insert(key));
    if !fresh {
        return;
    }
    let outcome = bridge.coverage_outcome(resolved, code);
    crate::trace::event(
        "dex3oat",
        format!("{outcome} {}", resolved.method.display_signature()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // cite: art reg_type_cache.cc#FromDescriptor
    #[test]
    fn descriptor_maps_to_the_right_verifier_category() {
        assert_eq!(reg_type_of_descriptor("I"), RegType::Int32);
        assert_eq!(reg_type_of_descriptor("Z"), RegType::Int32);
        assert_eq!(reg_type_of_descriptor("B"), RegType::Int32);
        assert_eq!(reg_type_of_descriptor("J"), RegType::LongLo);
        assert_eq!(reg_type_of_descriptor("D"), RegType::DoubleLo);
        assert_eq!(reg_type_of_descriptor("F"), RegType::Float);
        assert_eq!(
            reg_type_of_descriptor("Ljava/lang/String;"),
            RegType::Reference
        );
        assert_eq!(reg_type_of_descriptor("[I"), RegType::Reference);
    }
}

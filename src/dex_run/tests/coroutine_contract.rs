use super::super::*;

fn execute_guest_invoke_contract_method(name: &str, return_type: &str) -> DexExecutionResult {
    let dex_file = crate::dex::parse_dex_file(
        "coroutine_contract.dex",
        include_bytes!("fixtures/coroutine_contract.dex").to_vec(),
    )
    .expect("guest invoke contract DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/CoroutineContract;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: if return_type == "Z" { "Z" } else { "L" }.to_owned(),
            return_type: return_type.to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("guest invoke contract method should resolve")
        .expect("guest invoke contract method should exist");

    program
        .execute_method(None, &resolved, None, 0)
        .expect("guest invoke contract method should execute")
}

// cite: art interpreter_common.cc#DoCallCommon, DoCall
#[test]
fn art_do_call_copies_repeated_guest_arguments_without_host_substitution() {
    assert_eq!(
        execute_guest_invoke_contract_method("runWithContext", "Ljava/lang/Object;"),
        DexExecutionResult::Value(DexValue::String(std::rc::Rc::from("scope-context")))
    );
}

// cite: art interpreter_common.h#DoInvoke
// cite: art interpreter_common.cc#DoCall
#[test]
fn art_do_invoke_executes_guest_interface_callback() {
    assert_eq!(
        execute_guest_invoke_contract_method("runLaunch", "Z"),
        DexExecutionResult::Int(1)
    );
}

// cite: art interpreter_common.cc#MoveToExceptionHandler
#[test]
fn art_move_to_exception_handler_catches_guest_callee_throw() {
    assert_eq!(
        execute_guest_invoke_contract_method("catchWithContextThrow", "Ljava/lang/Object;"),
        DexExecutionResult::Value(DexValue::String(std::rc::Rc::from("caught-guest-throw")))
    );
}

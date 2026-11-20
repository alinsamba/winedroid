use super::*;

impl DexProgram {
    pub(super) fn missing_code_method(&self, method: &DexMethodId) -> Result<DexExecutionResult> {
        let error = WinedroidError::new(format!(
            "DEX method has no code item\nnext missing primitive: method {}.{}",
            method.class_descriptor,
            method.display_signature()
        ));
        if self.strict_missing {
            return Err(error);
        }
        match self.handle_missing_call(method, None, error)? {
            DexInvocationResult::Void => Ok(DexExecutionResult::Void),
            DexInvocationResult::Value(value) => Ok(DexExecutionResult::Value(value)),
            _ => Err(WinedroidError::new("invalid diagnostic execution result")),
        }
    }

    pub(super) fn handle_missing_call(
        &self,
        method: &DexMethodId,
        caller: Option<&str>,
        error: WinedroidError,
    ) -> Result<DexInvocationResult> {
        let signature = format!("{}.{}", method.class_descriptor, method.display_signature());
        let primitive = format!("method {signature}");
        let caller = caller.unwrap_or("<runtime entrypoint>");
        let reason = error.to_string();
        if self.strict_missing {
            if error.has_missing_context() {
                return Err(error);
            }
            let detail = format!(
                "{reason}\nmissing call: {signature}\ncaller class: {caller}\npolicy: strict"
            );
            crate::missing::annotate(error.missing_primitive().unwrap_or(&primitive), &detail);
            return Err(error.with_context(&format!(
                "missing call: {signature}\ncaller class: {caller}\npolicy: strict"
            )));
        }

        let result = diagnostic_return_value(&method.proto.return_type)?;
        let detail = format!(
            "abandoned call: {signature}\ncaller class: {caller}\npolicy: best-effort; call abandoned, side effects may be incomplete\nsubstitute: {result:?}\nreason: {reason}"
        );
        if error.missing_primitive() == Some(primitive.as_str()) {
            crate::missing::annotate(&primitive, &detail);
        } else if let Some(cause) = error.missing_primitive() {
            crate::missing::record_abandoned_call(cause, &signature, &detail);
        }
        Ok(result)
    }
}

fn diagnostic_return_value(descriptor: &str) -> Result<DexInvocationResult> {
    let value = match descriptor {
        "V" => return Ok(DexInvocationResult::Void),
        "Z" | "B" | "C" | "S" | "I" => DexValue::Int(0),
        "J" | "D" => DexValue::Long(0),
        "F" => DexValue::Float(0.0),
        descriptor if descriptor.starts_with('L') || descriptor.starts_with('[') => DexValue::Null,
        _ => {
            return Err(WinedroidError::new(format!(
                "invalid return descriptor for missing call: {descriptor}"
            )));
        }
    };
    Ok(DexInvocationResult::Value(value))
}

impl DexRunInvocationHandler<'_> {
    pub(super) fn finish_invocation(
        &self,
        method_index: u32,
        result: Result<DexInvocationResult>,
    ) -> Result<DexInvocationResult> {
        match result {
            Err(error) if error.missing_primitive().is_some() => {
                let method = self
                    .program
                    .method_id(self.current_dex_file_index, method_index)?;
                self.program.handle_missing_call(
                    &method,
                    self.current_class_descriptor.as_deref(),
                    error,
                )
            }
            result => result,
        }
    }
}

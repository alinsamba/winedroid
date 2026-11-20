use super::super::*;

impl DexProgram {
    pub(super) fn invoke_org_xml_sax_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor != "Lorg/xml/sax/Attributes;" {
            return None;
        }

        let invocation = invocation?;
        let key = map_identity_key(invocation.arguments.first()?);
        let attributes = self.runtime.borrow().sax_attributes.get(&key).cloned()?;

        match (method.name.as_str(), method.proto.descriptor().as_str()) {
            ("getLength", "()I") => Some(DexExecutionResult::Int(attributes.len() as i32)),
            ("getValue", "(Ljava/lang/String;)Ljava/lang/String;") => {
                let DexValue::String(name) = invocation.arguments.get(1)? else {
                    return None;
                };
                let value = attributes
                    .iter()
                    .find(|(attribute_name, _)| {
                        attribute_name.as_str() == name.as_ref()
                            || crate::binary_xml::local_name(attribute_name) == &**(name)
                    })
                    .map(|(_, value)| value.clone());
                Some(DexExecutionResult::Value(string_or_null(value)))
            }
            ("getValue", "(I)Ljava/lang/String;") => {
                let value = attribute_at(&attributes, invocation.arguments.get(1)?)
                    .map(|(_, value)| value.clone());
                Some(DexExecutionResult::Value(string_or_null(value)))
            }
            ("getQName", "(I)Ljava/lang/String;") => {
                let name = attribute_at(&attributes, invocation.arguments.get(1)?)
                    .map(|(name, _)| name.clone());
                Some(DexExecutionResult::Value(string_or_null(name)))
            }
            ("getLocalName", "(I)Ljava/lang/String;") => {
                let name = attribute_at(&attributes, invocation.arguments.get(1)?)
                    .map(|(name, _)| crate::binary_xml::local_name(name).to_owned());
                Some(DexExecutionResult::Value(string_or_null(name)))
            }
            ("getURI", "(I)Ljava/lang/String;") => Some(DexExecutionResult::Value(
                DexValue::String(std::rc::Rc::from(String::new())),
            )),
            _ => None,
        }
    }
}

fn attribute_at<'a>(
    attributes: &'a [(String, String)],
    index: &DexValue,
) -> Option<&'a (String, String)> {
    let DexValue::Int(index) = index else {
        return None;
    };
    usize::try_from(*index)
        .ok()
        .and_then(|index| attributes.get(index))
}

fn string_or_null(value: Option<String>) -> DexValue {
    value.map_or(DexValue::Null, |value| DexValue::String(value.into()))
}

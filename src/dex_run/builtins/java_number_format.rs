use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_number_format_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor != "Ljava/text/NumberFormat;" {
            return None;
        }

        if matches!(method.name.as_str(), "getInstance" | "getNumberInstance")
            && is_number_format_factory_proto(&method.proto.descriptor())
        {
            return Some(DexExecutionResult::Value(
                self.new_number_format(NumberFormatState::default()),
            ));
        }
        if method.name == "getIntegerInstance"
            && is_number_format_factory_proto(&method.proto.descriptor())
        {
            return Some(DexExecutionResult::Value(self.new_number_format(
                NumberFormatState {
                    minimum_fraction_digits: 0,
                    maximum_fraction_digits: 0,
                    grouping_used: true,
                },
            )));
        }

        if method.name == "setMaximumFractionDigits" && method.proto.descriptor_is("(I)V") {
            let invocation = invocation?;
            self.update_number_format(invocation, |state, digits| {
                state.maximum_fraction_digits = digits;
                state.minimum_fraction_digits = state.minimum_fraction_digits.min(digits);
            })?;
            return Some(DexExecutionResult::Void);
        }
        if method.name == "setMinimumFractionDigits" && method.proto.descriptor_is("(I)V") {
            let invocation = invocation?;
            self.update_number_format(invocation, |state, digits| {
                state.minimum_fraction_digits = digits;
                state.maximum_fraction_digits = state.maximum_fraction_digits.max(digits);
            })?;
            return Some(DexExecutionResult::Void);
        }
        if method.name == "setGroupingUsed" && method.proto.descriptor_is("(Z)V") {
            let invocation = invocation?;
            let DexValue::Int(used) = *invocation.arguments.get(1)? else {
                return None;
            };
            let key = map_identity_key(invocation.arguments.first()?);
            self.runtime
                .borrow_mut()
                .number_format_values
                .entry(key)
                .or_default()
                .grouping_used = used != 0;
            return Some(DexExecutionResult::Void);
        }

        if method.name == "format" && method.proto.descriptor_is("(D)Ljava/lang/String;") {
            let invocation = invocation?;
            let DexValue::Long(bits) = *invocation.arguments.get(1)? else {
                return None;
            };
            return Some(self.format_number_value(invocation, f64::from_bits(bits as u64)));
        }
        if method.name == "format" && method.proto.descriptor_is("(J)Ljava/lang/String;") {
            let invocation = invocation?;
            let DexValue::Long(value) = *invocation.arguments.get(1)? else {
                return None;
            };
            return Some(self.format_number_value(invocation, value as f64));
        }
        if method.name == "format"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.number_value_from_object(invocation.arguments.get(1)?)?;
            return Some(self.format_number_value(invocation, value));
        }

        if method.name == "parse"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/lang/Number;")
        {
            let invocation = invocation?;
            let DexValue::String(text) = invocation.arguments.get(1)? else {
                return None;
            };
            return Some(self.parse_number_value(text));
        }

        None
    }

    fn parse_number_value(&self, text: &str) -> DexExecutionResult {
        match parse_leading_number(text) {
            Some(value) => DexExecutionResult::Value(DexValue::Reference(std::rc::Rc::from(
                format!("Ljava/lang/Double;:{:016x}", value.to_bits()),
            ))),
            None => {
                let exception = self.allocate_object("Ljava/text/ParseException;".to_owned());
                self.write_stored_instance_field(
                    &exception,
                    &number_format_throwable_message_field(),
                    DexValue::String(std::rc::Rc::from(format!("Unparseable number: \"{text}\""))),
                );
                DexExecutionResult::Throw(exception)
            }
        }
    }

    fn number_value_from_object(&self, value: &DexValue) -> Option<f64> {
        match value {
            DexValue::Int(value) => Some(*value as f64),
            DexValue::Long(value) => Some(*value as f64),
            DexValue::Float(value) => Some(*value as f64),
            DexValue::Reference(reference) => {
                if let Some(bits) = boxed_double_reference_value(reference) {
                    Some(f64::from_bits(bits as u64))
                } else if let Some(bits) = boxed_float_reference_value(reference) {
                    Some(bits as f64)
                } else if let Some(value) = boxed_long_reference_value(reference) {
                    Some(value as f64)
                } else {
                    reference
                        .strip_prefix("Ljava/lang/Integer;:")?
                        .parse::<i64>()
                        .ok()
                        .map(|value| value as f64)
                }
            }
            DexValue::Object(_) => {
                for (descriptor, type_descriptor, name, as_double) in [
                    ("Ljava/lang/Double;", "D", "value", true),
                    ("Ljava/lang/Long;", "J", "value", false),
                    ("Ljava/lang/Integer;", "I", "value", false),
                    ("Ljava/lang/Float;", "F", "value", false),
                ] {
                    let field = DexFieldId {
                        class_descriptor: descriptor.to_owned(),
                        type_descriptor: type_descriptor.to_owned(),
                        name: name.to_owned(),
                    };
                    match self.read_stored_instance_field(value, &field) {
                        Some(DexValue::Long(bits)) if as_double => {
                            return Some(f64::from_bits(bits as u64));
                        }
                        Some(DexValue::Long(value)) => return Some(value as f64),
                        Some(DexValue::Int(value)) => return Some(value as f64),
                        Some(DexValue::Float(value)) => return Some(value as f64),
                        _ => {}
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn new_number_format(&self, state: NumberFormatState) -> DexValue {
        let id = {
            let mut runtime = self.runtime.borrow_mut();
            let id = runtime.next_object_id;
            runtime.next_object_id += 1;
            id
        };
        let reference =
            DexValue::Reference(std::rc::Rc::from(format!("Ljava/text/NumberFormat;:{id}")));
        self.runtime
            .borrow_mut()
            .number_format_values
            .insert(map_identity_key(&reference), state);
        reference
    }

    fn update_number_format(
        &self,
        invocation: &DexInvocation,
        apply: impl FnOnce(&mut NumberFormatState, u32),
    ) -> Option<()> {
        let DexValue::Int(digits) = *invocation.arguments.get(1)? else {
            return None;
        };
        let key = map_identity_key(invocation.arguments.first()?);
        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.number_format_values.entry(key).or_default();
        apply(state, digits.max(0) as u32);
        Some(())
    }

    fn format_number_value(&self, invocation: &DexInvocation, value: f64) -> DexExecutionResult {
        let state = invocation
            .arguments
            .first()
            .map(map_identity_key)
            .and_then(|key| {
                self.runtime
                    .borrow()
                    .number_format_values
                    .get(&key)
                    .cloned()
            })
            .unwrap_or_default();
        DexExecutionResult::Value(DexValue::String(std::rc::Rc::from(format_number(
            &state, value,
        ))))
    }
}

fn is_number_format_factory_proto(descriptor: &str) -> bool {
    matches!(
        descriptor,
        "()Ljava/text/NumberFormat;" | "(Ljava/util/Locale;)Ljava/text/NumberFormat;"
    )
}

fn number_format_throwable_message_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    }
}

fn parse_leading_number(text: &str) -> Option<f64> {
    let cleaned: String = text.chars().filter(|character| *character != ',').collect();
    let trimmed = cleaned.trim();
    let characters: Vec<char> = trimmed.chars().collect();

    for end in (1..=characters.len()).rev() {
        let candidate: String = characters[..end].iter().collect();
        if let Ok(value) = candidate.parse::<f64>() {
            return Some(value);
        }
    }

    None
}

fn format_number(state: &NumberFormatState, value: f64) -> String {
    if !value.is_finite() {
        return if value.is_nan() {
            "NaN".to_owned()
        } else if value > 0.0 {
            "\u{221e}".to_owned()
        } else {
            "-\u{221e}".to_owned()
        };
    }

    let max = state.maximum_fraction_digits as usize;
    let min = state
        .minimum_fraction_digits
        .min(state.maximum_fraction_digits) as usize;

    let rounded = format!("{value:.max$}");
    let negative = rounded.starts_with('-');
    let body = rounded.trim_start_matches('-');
    let (integer_part, fraction_part) = body.split_once('.').unwrap_or((body, ""));

    let mut fraction = fraction_part.to_owned();
    while fraction.len() > min && fraction.ends_with('0') {
        fraction.pop();
    }

    let integer = if state.grouping_used {
        group_integer_digits(integer_part)
    } else {
        integer_part.to_owned()
    };

    let all_zero =
        integer_part.bytes().all(|byte| byte == b'0') && fraction.bytes().all(|byte| byte == b'0');
    let mut out = String::new();
    if negative && !all_zero {
        out.push('-');
    }
    out.push_str(&integer);
    if !fraction.is_empty() {
        out.push('.');
        out.push_str(&fraction);
    }
    out
}

fn group_integer_digits(integer_part: &str) -> String {
    let len = integer_part.len();
    let mut out = String::with_capacity(len + len / 3);
    for (index, digit) in integer_part.char_indices() {
        if index > 0 && (len - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(min: u32, max: u32, grouping: bool) -> NumberFormatState {
        NumberFormatState {
            minimum_fraction_digits: min,
            maximum_fraction_digits: max,
            grouping_used: grouping,
        }
    }

    #[test]
    fn parses_leading_number_ignoring_grouping_and_trailing_text() {
        assert_eq!(parse_leading_number("1,234.5"), Some(1234.5));
        assert_eq!(parse_leading_number("2"), Some(2.0));
        assert_eq!(parse_leading_number("12abc"), Some(12.0));
        assert_eq!(parse_leading_number(" 3.5 "), Some(3.5));
        assert_eq!(parse_leading_number("abc"), None);
        assert_eq!(parse_leading_number(""), None);
    }

    #[test]
    fn formats_with_grouping_and_trimmed_fraction() {
        assert_eq!(format_number(&state(0, 3, true), 1234.5), "1,234.5");
        assert_eq!(format_number(&state(0, 3, true), 1234.0), "1,234");
        assert_eq!(format_number(&state(2, 2, true), 1234.5), "1,234.50");
        assert_eq!(
            format_number(&state(0, 2, false), 1234567.891),
            "1234567.89"
        );
    }

    #[test]
    fn rounds_and_handles_sign_and_zero() {
        assert_eq!(format_number(&state(0, 2, true), 1.005), "1");
        assert_eq!(format_number(&state(2, 2, true), -1234.5), "-1,234.50");
        assert_eq!(format_number(&state(2, 2, true), -0.0), "0.00");
        assert_eq!(format_number(&state(0, 0, true), 1999.9), "2,000");
    }
}

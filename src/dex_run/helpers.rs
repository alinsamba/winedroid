use crate::dex_interpreter::DexValue;

pub(super) fn normalize_class_name(class_name: &str) -> String {
    if class_name.starts_with('L') && class_name.ends_with(';') {
        return class_name.to_owned();
    }

    format!("L{};", class_name.replace('.', "/"))
}

pub(super) fn instance_field_owner_key(value: &DexValue) -> Option<String> {
    match value {
        DexValue::Object(value) => Some(format!("object:{}", value.id)),
        DexValue::Reference(value) => Some(format!("reference:{value}")),
        DexValue::Uninitialized
        | DexValue::Null
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_) => None,
    }
}

pub(super) fn map_identity_key(value: &DexValue) -> String {
    match value {
        DexValue::Object(value) => format!("object:{}", value.id),
        DexValue::Reference(value) => format!("reference:{value}"),
        DexValue::Null => "null".to_owned(),
        DexValue::Int(value) => format!("int:{value}"),
        DexValue::Long(value) => format!("long:{value}"),
        DexValue::Float(value) => format!("float:{:08x}", value.to_bits()),
        DexValue::String(value) => format!("string:{value}"),
        DexValue::Uninitialized => "uninitialized".to_owned(),
    }
}

pub(super) fn primitive_int_argument(value: &DexValue) -> Option<i32> {
    let DexValue::Int(value) = value else {
        return None;
    };

    Some(*value)
}

pub(super) fn identity_key_is_in_active_object_scope(value: &str, min_object_id: u64) -> bool {
    if min_object_id == 0 {
        return true;
    }

    identity_object_id(value)
        .map(|id| id >= min_object_id)
        .unwrap_or(false)
}

pub(super) fn identity_object_id(value: &str) -> Option<u64> {
    value
        .strip_prefix("object:")
        .and_then(|id| id.parse::<u64>().ok())
}

pub(super) fn object_identity_sort_key(value: &str) -> u64 {
    identity_object_id(value).unwrap_or(u64::MAX)
}

pub(super) fn descriptor_shorty(descriptor: &str) -> Option<char> {
    if descriptor.starts_with('L') || descriptor.starts_with('[') {
        return Some('L');
    }

    match descriptor {
        "V" => Some('V'),
        "Z" => Some('Z'),
        "B" => Some('B'),
        "S" => Some('S'),
        "C" => Some('C'),
        "I" => Some('I'),
        "J" => Some('J'),
        "F" => Some('F'),
        "D" => Some('D'),
        _ => None,
    }
}

pub(super) fn proto_shorty(return_type: &str, parameters: &[String]) -> Option<String> {
    let mut shorty = String::new();
    shorty.push(descriptor_shorty(return_type)?);

    for parameter in parameters {
        shorty.push(descriptor_shorty(parameter)?);
    }

    Some(shorty)
}

pub(super) fn descriptor_package_name(descriptor: &str) -> Option<String> {
    let java_name = descriptor_to_java_name(descriptor)?;
    let last_dot = java_name.rfind('.')?;

    Some(java_name[..last_dot].to_owned())
}

pub(super) fn stable_hash_i32(value: &str) -> i32 {
    let mut hash = 0i32;

    for byte in value.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(byte));
    }

    hash
}

pub(super) fn value_class_descriptor(value: &DexValue) -> Option<&str> {
    match value {
        DexValue::Object(object) => Some(object.class_descriptor.as_ref()),
        DexValue::Reference(reference) => reference_descriptor(reference),
        DexValue::String(_) => Some("Ljava/lang/String;"),
        DexValue::Null
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::Uninitialized => None,
    }
}

pub(super) fn reference_descriptor(reference: &str) -> Option<&str> {
    if reference.starts_with('L') {
        let end = reference.find(';')?;
        if end == 1 {
            return None;
        }
        return reference.get(..=end);
    }

    let component_index = reference.bytes().take_while(|byte| *byte == b'[').count();
    if component_index == 0 {
        return None;
    }

    let component = *reference.as_bytes().get(component_index)?;
    let descriptor_end = match component {
        b'L' => {
            let component_end = reference.get(component_index..)?.find(';')?;
            if component_end == 1 {
                return None;
            }
            component_end + component_index + 1
        }
        b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' => component_index + 1,
        _ => return None,
    };

    reference.get(..descriptor_end)
}

pub(super) fn descriptor_to_java_name(descriptor: &str) -> Option<String> {
    if descriptor.starts_with('L') && descriptor.ends_with(';') {
        let class_name = &descriptor[1..descriptor.len() - 1];
        return Some(class_name.replace('/', "."));
    }

    if descriptor.starts_with('[') {
        return Some(descriptor.replace('/', "."));
    }

    None
}

// cite: art descriptors_names.cc#AppendPrettyDescriptor, PrettyDescriptor
pub(super) fn descriptor_to_pretty_name(descriptor: &str) -> Option<String> {
    let dimensions = descriptor.bytes().take_while(|byte| *byte == b'[').count();
    let component = descriptor.get(dimensions..)?;
    let mut name = match component {
        "B" => "byte".to_owned(),
        "C" => "char".to_owned(),
        "D" => "double".to_owned(),
        "F" => "float".to_owned(),
        "I" => "int".to_owned(),
        "J" => "long".to_owned(),
        "S" => "short".to_owned(),
        "Z" => "boolean".to_owned(),
        "V" => "void".to_owned(),
        component if component.starts_with('L') && component.ends_with(';') => {
            component[1..component.len() - 1].replace('/', ".")
        }
        _ => return None,
    };

    for _ in 0..dimensions {
        name.push_str("[]");
    }

    Some(name)
}

pub(super) fn java_long_decode(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let (negative, unsigned) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = value.strip_prefix('+') {
        (false, rest)
    } else {
        (false, value)
    };

    let (radix, digits) = if let Some(rest) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        (16, rest)
    } else if let Some(rest) = unsigned.strip_prefix('#') {
        (16, rest)
    } else if unsigned.starts_with('0') && unsigned.len() > 1 {
        (8, &unsigned[1..])
    } else {
        (10, unsigned)
    };

    if digits.is_empty() {
        return None;
    }

    let parsed = i64::from_str_radix(digits, radix).ok()?;

    if negative {
        parsed.checked_neg()
    } else {
        Some(parsed)
    }
}

pub(super) fn utf16_units_before_byte(input: &str, byte_index: usize) -> Option<i32> {
    if byte_index > input.len() || !input.is_char_boundary(byte_index) {
        return None;
    }

    i32::try_from(input[..byte_index].encode_utf16().count()).ok()
}

pub(super) fn utf16_units_to_byte_index(input: &str, units: i32) -> Option<usize> {
    let target_units = usize::try_from(units).ok()?;

    if target_units == 0 {
        return Some(0);
    }

    let mut current_units = 0_usize;
    for (byte_index, character) in input.char_indices() {
        if current_units == target_units {
            return Some(byte_index);
        }

        current_units += character.len_utf16();

        if current_units == target_units {
            return Some(byte_index + character.len_utf8());
        }

        if current_units > target_units {
            return None;
        }
    }

    (current_units == target_units).then_some(input.len())
}

pub(super) fn utf16_string_slice(value: &str, start: i32, end: i32) -> Option<String> {
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    let units: Vec<u16> = value.encode_utf16().collect();

    if start > end || end > units.len() {
        return None;
    }

    String::from_utf16(&units[start..end]).ok()
}

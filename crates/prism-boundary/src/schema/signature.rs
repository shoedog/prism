use crate::inventory::{EnumValue, FieldShape, Property, SchemaIndex};
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub fn signature(shape: &FieldShape) -> String {
    let properties = [
        tagged(&shape.field_type, |s| string(s)),
        tagged(&shape.format, |s| string(s)),
        tagged(&shape.required, bool::to_string),
        tagged(&shape.enum_values, |values| {
            let mut values: Vec<_> = values.iter().map(member).collect();
            values.sort();
            values.dedup();
            format!("[{}]", values.join(","))
        }),
        tagged(&shape.item_type, |s| string(s)),
    ];
    format!("[{}]", properties.join(","))
}

fn tagged<T>(property: &Property<T>, encode: impl FnOnce(&T) -> String) -> String {
    match property {
        Property::Missing => r#"["missing"]"#.into(),
        Property::BranchDependent => r#"["branch_dependent"]"#.into(),
        Property::Unknown => r#"["unknown"]"#.into(),
        Property::Known(value) => format!(r#"["known",{}]"#, encode(value)),
    }
}

pub(super) fn content_hash(index: &SchemaIndex) -> String {
    let mut records = Vec::new();
    for operation in &index.operations {
        let id = operation.id.to_string();
        let encoded = string(&id);
        records.push((id.clone(), format!(r#"["op",{encoded}]"#)));
        for key in &operation.success_keys {
            let key = string(&key.to_string());
            records.push((id.clone(), format!(r#"["status",{encoded},{key}]"#)));
        }
    }
    for field in &index.fields {
        let id = field.id.to_string();
        let record = format!(r#"["field",{},{}]"#, string(&id), signature(&field.shape));
        records.push((id, record));
    }
    records.sort_unstable();
    records.dedup();
    let mut hash = Sha256::new();
    for (_, record) in records {
        hash.update(record);
        hash.update(b"\n");
    }
    format!("{:x}", hash.finalize())
}

pub(super) fn member(value: &EnumValue) -> String {
    match value {
        EnumValue::Null => "null".into(),
        EnumValue::Boolean(value) => value.to_string(),
        EnumValue::Number(value) => canonical_number(value),
        EnumValue::String(value) => string(value),
    }
}

fn string(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' | '\\' => {
                output.push('\\');
                output.push(ch);
            }
            '\0'..='\u{1f}' => write!(output, "\\u{:04x}", u32::from(ch)).unwrap(),
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

pub(super) fn canonical_number(literal: &str) -> String {
    let (mantissa, exponent) = literal.split_once(['e', 'E']).unwrap_or((literal, "0"));
    let (sign, digits) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |s| ("-", s));
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let coefficient = format!("{whole}{fraction}");
    let significant = coefficient.trim_matches('0');
    if significant.is_empty() {
        return "0".into();
    }
    let removed = coefficient.len() - coefficient.trim_end_matches('0').len();
    let offset = removed as i64 - fraction.len() as i64;
    let exponent = shift_exponent(exponent, offset);
    if exponent == "0" {
        format!("{sign}{significant}")
    } else {
        format!("{sign}{significant}e{exponent}")
    }
}

// The exponent may itself exceed a machine integer. In that case adding an
// offset of at most the literal length cannot change its sign.
pub(super) fn shift_exponent(exponent: &str, offset: i64) -> String {
    if let Ok(value) = exponent.parse::<i128>() {
        if let Some(sum) = value.checked_add(i128::from(offset)) {
            return sum.to_string();
        }
    }
    let negative = exponent.starts_with('-');
    let mut carry = if negative {
        -i128::from(offset)
    } else {
        i128::from(offset)
    };
    let mut digits = Vec::new();
    for digit in exponent.trim_start_matches(['+', '-']).bytes().rev() {
        let sum = i128::from(digit - b'0') + carry;
        digits.push(b'0' + sum.rem_euclid(10) as u8);
        carry = sum.div_euclid(10);
    }
    while carry > 0 {
        digits.push(b'0' + (carry % 10) as u8);
        carry /= 10;
    }
    while digits.last() == Some(&b'0') {
        digits.pop();
    }
    if negative {
        digits.push(b'-');
    }
    digits.reverse();
    String::from_utf8(digits).unwrap()
}

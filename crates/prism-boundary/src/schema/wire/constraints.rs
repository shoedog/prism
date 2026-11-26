use super::super::signature::{canonical_number, member, shift_exponent};
use super::{Dialect, Node};
use crate::inventory::{EnumValue, Property};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const OBJECT: u8 = 1;
pub(super) const ARRAY: u8 = 2;
const STRING: u8 = 4;
const INTEGER: u8 = 8;
const FRACTION: u8 = 16;
const NUMBER: u8 = INTEGER | FRACTION;
const BOOLEAN: u8 = 32;
pub(super) const NULL: u8 = 64;
pub(super) const ALL: u8 = OBJECT | ARRAY | STRING | NUMBER | BOOLEAN | NULL;

pub(super) fn type_bits(name: &str) -> Option<u8> {
    Some(match name {
        "object" => OBJECT,
        "array" => ARRAY,
        "string" => STRING,
        "integer" => INTEGER,
        "number" => NUMBER,
        "boolean" => BOOLEAN,
        "null" => NULL,
        _ => return None,
    })
}

pub(super) fn types(nodes: &[Node<'_, '_>], dialect: Dialect) -> (u8, Property<Vec<String>>) {
    let mut types = ALL;
    let mut explicit = false;
    let mut inferred = 0;
    for node in nodes {
        if node.value == &Value::Bool(false) {
            types = 0;
        }
        if let Some(value) = node.get("type") {
            explicit = true;
            let mut selected = value.as_str().and_then(type_bits).unwrap_or_default();
            if let Some(values) = value.as_array() {
                for name in values
                    .iter()
                    .filter_map(Value::as_str)
                    .filter_map(type_bits)
                {
                    selected |= name;
                }
            }
            if dialect == Dialect::OpenApi30 && node.get("nullable") == Some(&Value::Bool(true)) {
                selected |= NULL;
            }
            types &= selected;
        }
        if [
            "properties",
            "required",
            "additionalProperties",
            "minProperties",
            "maxProperties",
        ]
        .iter()
        .any(|key| node.get(key).is_some())
        {
            inferred |= OBJECT;
        }
        if node.get("items").is_some() {
            inferred |= ARRAY;
        }
    }
    if explicit {
        return (types, type_set(types));
    }
    let shape = match inferred {
        OBJECT => Property::Known(vec!["object".into()]),
        ARRAY => Property::Known(vec!["array".into()]),
        0 => Property::Missing,
        _ => Property::BranchDependent,
    };
    (types, shape)
}

pub(super) fn type_set(types: u8) -> Property<Vec<String>> {
    if types == 0 {
        return Property::BranchDependent;
    }
    let names = [
        (ARRAY, "array"),
        (BOOLEAN, "boolean"),
        (INTEGER, "integer"),
        (NULL, "null"),
        (FRACTION, "number"),
        (OBJECT, "object"),
        (STRING, "string"),
    ]
    .into_iter()
    .filter(|&(bit, _)| types & bit != 0 && (bit != INTEGER || types & FRACTION == 0))
    .map(|(_, name)| name.into())
    .collect();
    Property::Known(names)
}

pub(super) fn limits(nodes: &[Node<'_, '_>], min: &str, max: &str) -> (u64, u64) {
    let values = |key| {
        nodes
            .iter()
            .filter_map(move |node| node.get(key).and_then(nonnegative))
    };
    (
        values(min).max().unwrap_or(0),
        values(max).min().unwrap_or(u64::MAX),
    )
}

pub(super) fn nonnegative(value: &Value) -> Option<u64> {
    let canonical = canonical_number(&value.as_number()?.to_string());
    let (digits, exponent) = parts(&canonical);
    if digits.starts_with('-') || exponent < 0 {
        return None;
    }
    let digits = digits.parse::<u64>().ok();
    let power = u32::try_from(exponent)
        .ok()
        .and_then(|exp| 10u64.checked_pow(exp));
    digits?.checked_mul(power?)
}

fn scalar(value: &Value) -> Option<EnumValue> {
    Some(match value {
        Value::Null => EnumValue::Null,
        Value::Bool(value) => EnumValue::Boolean(*value),
        Value::String(value) => EnumValue::String(value.clone()),
        Value::Number(value) => EnumValue::Number(canonical_number(&value.to_string())),
        _ => return None,
    })
}

pub(super) fn domain(nodes: &[Node<'_, '_>]) -> Option<BTreeMap<String, EnumValue>> {
    let mut domain: Option<BTreeMap<_, _>> = None;
    for node in nodes {
        for values in [
            node.get("enum")
                .and_then(Value::as_array)
                .map(Vec::as_slice),
            node.get("const").map(std::slice::from_ref),
        ]
        .into_iter()
        .flatten()
        {
            let values: BTreeMap<_, _> = values
                .iter()
                .filter_map(scalar)
                .map(|value| (member(&value), value))
                .collect();
            if let Some(existing) = &mut domain {
                existing.retain(|key, _| values.contains_key(key));
            } else {
                domain = Some(values);
            }
        }
    }
    domain
}

pub(super) struct Feasibility {
    pub types: u8,
    pub witnesses: u8,
    pub enum_values: Property<Vec<EnumValue>>,
    pub unknown: bool,
}

pub(super) fn feasible(nodes: &[Node<'_, '_>], mut types: u8, dialect: Dialect) -> Feasibility {
    let present = nodes.iter().any(|node| node.get("enum").is_some());
    let (min_length, max_length) = limits(nodes, "minLength", "maxLength");
    if min_length > max_length {
        types &= !STRING;
    }
    let (low, high) = bounds(nodes, dialect);
    let mut unchecked = 0;
    if let (Some(low), Some(high)) = (&low, &high) {
        match compare(&low.value, &high.value) {
            Ordering::Greater => types &= !NUMBER,
            Ordering::Equal if low.exclusive || high.exclusive => types &= !NUMBER,
            _ => {
                if fraction(&low.value) || fraction(&high.value) || low.exclusive || high.exclusive
                {
                    match integer_in_interval(low, high) {
                        Some(false) => types &= !INTEGER,
                        None if types & FRACTION == 0 => unchecked |= INTEGER,
                        _ => {}
                    }
                }
            }
        }
    }
    let patterns: Vec<_> = nodes
        .iter()
        .filter_map(|node| node.get("pattern").and_then(Value::as_str))
        .collect();
    let multiples: Vec<_> = nodes
        .iter()
        .filter_map(|node| node.get("multipleOf").and_then(Value::as_number))
        .map(|value| canonical_number(&value.to_string()))
        .collect();
    let mut enum_values = Property::Missing;
    let witnesses;
    if let Some(mut values) = domain(nodes) {
        let mut types_in_domain = 0;
        let mut proven = 0;
        unchecked = 0;
        values.retain(|_, value| {
            let mut uncertain = false;
            let bits = match value {
                EnumValue::Null => NULL,
                EnumValue::Boolean(_) => BOOLEAN,
                EnumValue::String(s) => {
                    let length = s.chars().count() as u64;
                    if length < min_length || length > max_length {
                        return false;
                    }
                    for pattern in &patterns {
                        match literal_pattern(pattern, s) {
                            Some(false) => return false,
                            None => uncertain = true,
                            _ => {}
                        }
                    }
                    STRING
                }
                EnumValue::Number(value) => {
                    if low
                        .as_ref()
                        .is_some_and(|bound| bound.rejects(value, Ordering::Less))
                        || high
                            .as_ref()
                            .is_some_and(|bound| bound.rejects(value, Ordering::Greater))
                    {
                        return false;
                    }
                    for divisor in &multiples {
                        match divisible(value, divisor) {
                            Some(false) => return false,
                            None => uncertain = true,
                            _ => {}
                        }
                    }
                    if fraction(value) {
                        FRACTION
                    } else {
                        INTEGER
                    }
                }
            };
            let admitted = bits & types;
            types_in_domain |= admitted;
            if uncertain {
                unchecked |= admitted;
            } else {
                proven |= admitted;
            }
            admitted != 0
        });
        types &= types_in_domain;
        witnesses = proven;
        if present {
            enum_values = Property::Known(values.into_values().collect());
        }
    } else {
        if !patterns.is_empty() {
            unchecked |= STRING;
        }
        if !multiples.is_empty() {
            unchecked |= NUMBER;
        }
        witnesses = types & !unchecked;
    }
    Feasibility {
        types,
        witnesses,
        enum_values,
        unknown: types & unchecked != 0,
    }
}

struct Bound {
    value: String,
    exclusive: bool,
}

impl Bound {
    fn rejects(&self, value: &str, outside: Ordering) -> bool {
        let order = compare(value, &self.value);
        order == outside || order == Ordering::Equal && self.exclusive
    }
}

fn bounds(nodes: &[Node<'_, '_>], dialect: Dialect) -> (Option<Bound>, Option<Bound>) {
    let select = |inclusive: &str, exclusive: &str, outside| {
        let mut tightest: Option<Bound> = None;
        for node in nodes {
            let values = [
                (
                    node.get(inclusive),
                    dialect != Dialect::OpenApi31
                        && node.get(exclusive) == Some(&Value::Bool(true)),
                ),
                (
                    node.get(exclusive)
                        .filter(|_| dialect == Dialect::OpenApi31),
                    true,
                ),
            ];
            for (value, exclusive) in values {
                if let Some(value) = value.and_then(Value::as_number) {
                    let value = canonical_number(&value.to_string());
                    if tightest
                        .as_ref()
                        .is_none_or(|last| match compare(&value, &last.value) {
                            Ordering::Equal => exclusive,
                            order => order == outside,
                        })
                    {
                        tightest = Some(Bound { value, exclusive });
                    }
                }
            }
        }
        tightest
    };
    (
        select("minimum", "exclusiveMinimum", Ordering::Greater),
        select("maximum", "exclusiveMaximum", Ordering::Less),
    )
}

fn parts(value: &str) -> (&str, i128) {
    value
        .split_once('e')
        .map_or((value, 0), |(digits, exponent)| {
            (
                digits,
                exponent.parse().unwrap_or_else(|_| {
                    if exponent.starts_with('-') {
                        i128::MIN
                    } else {
                        i128::MAX
                    }
                }),
            )
        })
}

fn fraction(value: &str) -> bool {
    parts(value).1 < 0
}

fn compare(left: &str, right: &str) -> Ordering {
    if left == right {
        return Ordering::Equal;
    }
    if left == "0" || right == "0" || left.starts_with('-') != right.starts_with('-') {
        let sign = |value: &str| {
            if value == "0" {
                0
            } else if value.starts_with('-') {
                -1
            } else {
                1
            }
        };
        return sign(left).cmp(&sign(right));
    }
    let (digits_a, exponent_a) = left
        .trim_start_matches('-')
        .split_once('e')
        .unwrap_or((left.trim_start_matches('-'), "0"));
    let (digits_b, exponent_b) = right
        .trim_start_matches('-')
        .split_once('e')
        .unwrap_or((right.trim_start_matches('-'), "0"));
    let magnitude = compare_exponents(
        &shift_exponent(exponent_a, digits_a.len() as i64),
        &shift_exponent(exponent_b, digits_b.len() as i64),
    )
    .then_with(|| digits_a.cmp(digits_b));
    if left.starts_with('-') {
        magnitude.reverse()
    } else {
        magnitude
    }
}

fn compare_exponents(left: &str, right: &str) -> Ordering {
    match (left.starts_with('-'), right.starts_with('-')) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (true, true) => right.len().cmp(&left.len()).then_with(|| right.cmp(left)),
        (false, false) => left.len().cmp(&right.len()).then_with(|| left.cmp(right)),
    }
}

fn floor_ceil(value: &str) -> Option<(i128, i128)> {
    let (coefficient, exponent) = parts(value);
    let digits = coefficient.trim_start_matches('-');
    if exponent >= 0 {
        let power = 10i128.checked_pow(u32::try_from(exponent).ok()?)?;
        let integer = coefficient.parse::<i128>().ok()?.checked_mul(power)?;
        return Some((integer, integer));
    }
    let count = (digits.len() as i128 + exponent).max(0) as usize;
    let integer = if count == 0 {
        0
    } else {
        digits[..count].parse::<i128>().ok()?
    };
    if coefficient.starts_with('-') {
        Some((integer.checked_add(1)?.checked_neg()?, -integer))
    } else {
        Some((integer, integer.checked_add(1)?))
    }
}

fn integer_in_interval(low: &Bound, high: &Bound) -> Option<bool> {
    let (low_floor, low_ceil) = floor_ceil(&low.value)?;
    let (high_floor, high_ceil) = floor_ceil(&high.value)?;
    let min = if low.exclusive {
        low_floor.checked_add(1)?
    } else {
        low_ceil
    };
    let max = if high.exclusive {
        high_ceil.checked_sub(1)?
    } else {
        high_floor
    };
    Some(min <= max)
}

fn divisible(value: &str, divisor: &str) -> Option<bool> {
    if value == "0" {
        return Some(true);
    }
    let (value, value_exp) = parts(value);
    let (divisor, divisor_exp) = parts(divisor);
    let exponent = value_exp.checked_sub(divisor_exp)?;
    let mut value = value.parse::<i128>().ok()?;
    let mut divisor = divisor.parse::<i128>().ok()?;
    let scale = 10i128.checked_pow(u32::try_from(exponent.unsigned_abs()).ok()?)?;
    if exponent < 0 {
        divisor = divisor.checked_mul(scale)?;
    } else {
        value = value.checked_mul(scale)?;
    }
    value.checked_rem(divisor).map(|remainder| remainder == 0)
}

// Literal matches are shared by the regex dialects. Other patterns remain unchecked.
fn literal_pattern(pattern: &str, value: &str) -> Option<bool> {
    let start = pattern.starts_with('^');
    let end = pattern.ends_with('$');
    let literal = pattern.strip_prefix('^').unwrap_or(pattern);
    let literal = literal.strip_suffix('$').unwrap_or(literal);
    if literal.contains([
        '^', '$', '.', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '\\',
    ]) {
        return None;
    }
    Some(match (start, end) {
        (true, true) => value == literal,
        (true, false) => value.starts_with(literal),
        (false, true) => value.ends_with(literal),
        (false, false) => value.contains(literal),
    })
}

pub(super) fn required<'r>(node: &Node<'r, '_>) -> impl Iterator<Item = String> + 'r {
    node.get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
}

pub(super) fn valid_required(value: &Value, dialect: Dialect) -> bool {
    value.as_array().is_some_and(|items| {
        (dialect == Dialect::OpenApi31 || !items.is_empty())
            && items.iter().all(Value::is_string)
            && items
                .iter()
                .filter_map(Value::as_str)
                .collect::<BTreeSet<_>>()
                .len()
                == items.len()
    })
}

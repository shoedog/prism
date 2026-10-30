//! P1 supported config syntax. Unsupported input declines, preserving base.
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::fmt;
struct Unique;
impl<'de> Visitor<'de> for Unique {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("JSON without duplicate keys")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut m = serde_json::Map::new();
        while let Some(k) = a.next_key::<String>()? {
            if m.contains_key(&k) {
                return Err(serde::de::Error::custom("duplicate key"));
            }
            m.insert(k, a.next_value_seed(Unique)?);
        }
        Ok(Value::Object(m))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut v = Vec::new();
        while let Some(x) = a.next_element_seed(Unique)? {
            v.push(x);
        }
        Ok(Value::Array(v))
    }
    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Value, E> {
        Ok(Value::String(s.into()))
    }
    fn visit_bool<E: serde::de::Error>(self, b: bool) -> Result<Value, E> {
        Ok(Value::Bool(b))
    }
    fn visit_i64<E: serde::de::Error>(self, n: i64) -> Result<Value, E> {
        Ok(n.into())
    }
    fn visit_u64<E: serde::de::Error>(self, n: u64) -> Result<Value, E> {
        Ok(n.into())
    }
    fn visit_f64<E: serde::de::Error>(self, n: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(n)
            .map(Value::Number)
            .ok_or_else(|| E::custom("number"))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
}
impl<'de> DeserializeSeed<'de> for Unique {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        d.deserialize_any(self)
    }
}
pub(crate) fn jsonc(bytes: &[u8]) -> Option<Value> {
    let mut b = bytes.to_vec();
    let mut i = 0;
    let mut quoted = false;
    while i < b.len() {
        if quoted {
            if b[i] == b'\\' {
                i += 2;
                continue;
            }
            if b[i] == b'"' {
                quoted = false;
            }
            i += 1;
            continue;
        }
        if b[i] == b'"' {
            quoted = true;
            i += 1;
            continue;
        }
        if b.get(i..i + 2) == Some(b"//") {
            while i < b.len() && b[i] != b'\n' {
                b[i] = b' ';
                i += 1;
            }
            continue;
        }
        if b.get(i..i + 2) == Some(b"/*") {
            b[i] = b' ';
            b[i + 1] = b' ';
            i += 2;
            while i + 1 < b.len() && &b[i..i + 2] != b"*/" {
                b[i] = b' ';
                i += 1;
            }
            if i + 1 >= b.len() {
                return None;
            }
            b[i] = b' ';
            b[i + 1] = b' ';
            i += 2;
            continue;
        }
        i += 1;
    }
    i = 0;
    quoted = false;
    while i < b.len() {
        if quoted {
            if b[i] == b'\\' {
                i += 2;
                continue;
            }
            if b[i] == b'"' {
                quoted = false;
            }
        } else if b[i] == b'"' {
            quoted = true;
        } else if b[i] == b',' {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if b.get(j).is_some_and(|x| *x == b'}' || *x == b']') {
                b[i] = b' ';
            }
        }
        i += 1;
    }
    let mut d = serde_json::Deserializer::from_slice(&b);
    let v = Unique.deserialize(&mut d).ok()?;
    d.end().ok()?;
    Some(v)
}
pub(crate) fn norm(base: &str, p: &str) -> Option<String> {
    if p.starts_with('/') || p.contains('\\') || p.contains(':') {
        return None;
    }
    let joined = if base.is_empty() {
        p.into()
    } else {
        format!("{base}/{p}")
    };
    let mut out = Vec::new();
    for s in joined.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            _ => out.push(s),
        }
    }
    Some(out.join("/"))
}
pub(crate) fn dir(p: &str) -> &str {
    p.rsplit_once('/').map_or("", |(d, _)| d)
}
fn seg_match(p: &[u8], s: &[u8]) -> bool {
    let mut previous = vec![false; s.len() + 1];
    previous[0] = true;
    for &token in p {
        let mut next = vec![false; s.len() + 1];
        next[0] = token == b'*' && previous[0];
        for j in 1..=s.len() {
            next[j] = if token == b'*' {
                previous[j] || next[j - 1]
            } else {
                previous[j - 1] && (token == b'?' || token == s[j - 1])
            };
        }
        previous = next;
    }
    previous[s.len()]
}
fn path_match(p: &[&str], s: &[&str]) -> bool {
    let mut previous = vec![false; s.len() + 1];
    previous[0] = true;
    for &part in p {
        let mut next = vec![false; s.len() + 1];
        next[0] = part == "**" && previous[0];
        for j in 1..=s.len() {
            next[j] = if part == "**" {
                previous[j] || next[j - 1]
            } else {
                previous[j - 1] && seg_match(part.as_bytes(), s[j - 1].as_bytes())
            };
        }
        previous = next;
    }
    previous[s.len()]
}
fn matches(pattern: &str, file: &str, exclude: bool) -> Option<bool> {
    if !pattern.is_ascii()
        || !file.is_ascii()
        || pattern.len() > 512
        || file.len() > 4096
        || pattern.contains(['[', ']', '{', '}', '\\', ':'])
        || pattern.ends_with("/**")
        || pattern.split('/').any(|s| s.contains("**") && s != "**")
    {
        return None;
    }
    if !exclude
        && file
            .split('/')
            .any(|s| ["node_modules", "bower_components", "jspm_packages"].contains(&s))
    {
        return None;
    }
    let p = if !exclude
        && !pattern.contains(['*', '?'])
        && !pattern.rsplit('/').next()?.contains('.')
    {
        format!("{pattern}/**/*")
    } else {
        pattern.into()
    };
    Some(path_match(
        &p.split('/').collect::<Vec<_>>(),
        &file.split('/').collect::<Vec<_>>(),
    ))
}

pub(crate) fn pattern_matches(pattern: &str, file: &str) -> Option<bool> {
    matches(pattern, file, false)
}
pub(crate) fn exclude_matches(pattern: &str, file: &str) -> Option<bool> {
    let mut prefix = file;
    loop {
        if matches(pattern, prefix, true)? {
            return Some(true);
        }
        let Some((parent, _)) = prefix.rsplit_once('/') else {
            return Some(false);
        };
        prefix = parent;
    }
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Origin {
    pub scheme: Option<String>,
    pub authority: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::schema) struct Uri {
    pub origin: Origin,
    pub(super) path: String,
    query: Option<String>,
}

impl Uri {
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.split('#').next()?;
        if raw
            .bytes()
            .any(|b| b.is_ascii_control() || matches!(b, b' ' | b'\\' | b'{' | b'}'))
        {
            return None;
        }
        let (address, query) = raw
            .split_once('?')
            .map_or((raw, None), |(a, q)| (a, Some(q.into())));
        let (scheme, address) = match address.split('/').next()?.split_once(':') {
            Some((scheme, _)) if valid_scheme(scheme) => (
                Some(scheme.to_ascii_lowercase()),
                &address[scheme.len() + 1..],
            ),
            Some(_) => return None,
            None => (None, address),
        };
        let (authority, path) = if let Some(rest) = address.strip_prefix("//") {
            let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
            if authority.is_empty() && scheme.as_deref() != Some("file") {
                return None;
            }
            if authority.contains('@') {
                return None;
            }
            (Some(authority.to_ascii_lowercase()), path)
        } else {
            if scheme.is_some() && !address.starts_with('/') {
                return None;
            }
            (None, address)
        };
        Some(Self {
            origin: Origin { scheme, authority },
            path: path.into(),
            query,
        })
    }

    pub fn absolute(raw: &str) -> Option<Self> {
        let uri = Self::parse(raw)?.resolve(None)?;
        (uri.origin.scheme.is_some()).then_some(uri)
    }

    pub fn resolve(mut self, base: Option<&Self>) -> Option<Self> {
        if self.origin.scheme.is_some() || self.origin.authority.is_some() {
            if self.origin.scheme.is_none() {
                self.origin.scheme = base.and_then(|uri| uri.origin.scheme.clone());
            }
            self.path = normalize_absolute(&self.path);
        } else {
            let local_path = self.path.starts_with('/');
            if let Some(base) = base {
                self.origin = base.origin.clone();
                if self.path.is_empty() {
                    self.path.clone_from(&base.path);
                    if self.query.is_none() {
                        self.query.clone_from(&base.query);
                    }
                } else if !local_path {
                    let directory = base.path.rsplit_once('/').map_or("", |(parent, _)| parent);
                    self.path = format!("{directory}/{}", self.path);
                }
            } else if !local_path {
                return None;
            }
            self.path = normalize_absolute(&self.path);
        }
        Some(self)
    }

    pub fn key(&self) -> String {
        let mut key = self
            .origin
            .scheme
            .as_ref()
            .map_or(String::new(), |s| format!("{s}:"));
        if let Some(authority) = &self.origin.authority {
            key.push_str("//");
            key.push_str(authority);
        }
        key.push_str(&self.path);
        if let Some(query) = &self.query {
            key.push('?');
            key.push_str(query);
        }
        key
    }
}

pub(super) fn valid_scheme(scheme: &str) -> bool {
    scheme
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b))
}

fn normalize_absolute(path: &str) -> String {
    let Some(suffix) = path.strip_prefix('/') else {
        return "/".into();
    };
    let mut segments = Vec::new();
    for segment in suffix.split('/') {
        match segment {
            ".." => {
                segments.pop();
            }
            "." => {}
            value => segments.push(value),
        }
    }
    if path.ends_with("/..") || path.ends_with("/.") {
        segments.push("");
    }
    format!("/{}", segments.join("/"))
}

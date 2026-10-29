#[cfg(test)]
fn helper() -> u8 { 1 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t() { assert_eq!(helper(), 1); }
}

pub fn target(x: i32) -> bool {
    x > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_target() {
        target(1);
    }
}

pub fn target() -> i32 {
    1
}

mod tests {
    use crate::*;

    fn calls_target() -> i32 {
        target()
    }
}

pub fn target() -> i32 {
    1
}

mod outer {
    pub mod tests {
        use super::super::*;

        fn calls_target() -> i32 {
            target()
        }
    }
}

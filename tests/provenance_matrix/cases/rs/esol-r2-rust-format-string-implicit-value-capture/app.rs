fn main() -> Result<(), std::env::VarError> {
    let mode = std::env::var("MODE")?;
    println!("{mode}");
    Ok(())
}

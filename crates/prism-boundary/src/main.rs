use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Typed service boundary document contracts")]
struct Cli {}

fn main() {
    Cli::parse();
}

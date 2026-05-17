use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "run-all-now",
    version,
    about = "Ultra-fast native replacement for npm-run-all"
)]
struct Cli {
    #[arg(long, hide = true)]
    internal_smoke_test: bool,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    if cli.internal_smoke_test {
        println!("run-all-now smoke ok");
        return;
    }

    println!("run-all-now: native scaffold ready.");
    println!("Target replacement: npm-run-all.");
    println!("See docs/implementation-roadmap.md for the MVP plan.");

    if !cli.args.is_empty() {
        eprintln!(
            "warning: compatibility execution is not implemented yet; received {} argument(s).",
            cli.args.len()
        );
    }
}

use std::path::PathBuf;

use clap::Parser;

/// Generate Open Graphene protocol specifications from source inputs.
#[derive(Debug, Parser)]
#[command(name = "open-graphene-spec-gen")]
#[command(about = "Generate Open Graphene protocol specifications")]
struct Args {
    /// Path to an Open Graphene generator config file.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Print version and exit.
    #[arg(long)]
    version: bool,
}

fn main() {
    let args = Args::parse();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if let Some(config) = args.config {
        println!(
            "open-graphene-spec-gen accepted config: {}",
            config.display()
        );
        eprintln!("extractor implementation is pending; no files were generated.");
        return;
    }

    eprintln!(
        "open-graphene-spec-gen is a placeholder crate; pass --config <path> once extractor design is ready."
    );
}

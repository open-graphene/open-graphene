use clap::Parser;

/// Generate Open Graphene protocol specifications from source inputs.
#[derive(Debug, Parser)]
#[command(name = "open-graphene-spec-gen")]
#[command(about = "Generate Open Graphene protocol specifications")]
struct Args {
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

    eprintln!("open-graphene-spec-gen is a placeholder crate; extractor design is pending.");
}

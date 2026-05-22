use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use open_graphene_spec_gen::generate_from_config;

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

fn main() -> ExitCode {
    let args = Args::parse();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let Some(config) = args.config else {
        eprintln!("missing required --config <path>");
        return ExitCode::from(2);
    };

    match generate_from_config(&config) {
        Ok(result) => {
            println!("wrote {}", result.output_path.display());
            println!(
                "configured {} RPC APIs and {} selected RPC methods",
                result.rpc_api_count, result.selected_method_count
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

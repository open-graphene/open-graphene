use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use open_graphene_gen_bindings_rs::generate_bindings;

/// Generate Rust bindings from Open Graphene protocol specifications.
#[derive(Debug, Parser)]
#[command(name = "open-graphene-gen-bindings-rs")]
#[command(about = "Generate Rust bindings from Open Graphene protocol specifications")]
struct Args {
    /// Path to an Open Graphene protocol JSON spec.
    #[arg(long)]
    spec: PathBuf,

    /// Directory where generated Rust modules should be written.
    #[arg(long)]
    out_dir: PathBuf,
}

fn main() -> ExitCode {
    let args = Args::parse();

    match generate_bindings(&args.spec, &args.out_dir) {
        Ok(result) => {
            println!("wrote {}", result.output_path.display());
            println!(
                "loaded schema version {} for chain {} with {} RPC methods, {} structs, and {} operations",
                result.schema_version,
                result.chain_id,
                result.rpc_method_count,
                result.struct_count,
                result.operation_count,
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

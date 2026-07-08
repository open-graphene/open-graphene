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
                "configured {} RPC APIs, {} selected RPC methods, discovered {} source files, extracted {} FC_API blocks, extracted {} classes with {} method declarations and {} field declarations, extracted {} static variants, resolved {} RPC methods, {} structs, and {} static variants, {} diagnostics",
                result.rpc_api_count,
                result.selected_method_count,
                result.source_file_count,
                result.fc_api_count,
                result.class_count,
                result.method_declaration_count,
                result.field_declaration_count,
                result.static_variant_count,
                result.resolved_rpc_method_count,
                result.resolved_struct_count,
                result.resolved_static_variant_count,
                result.diagnostic_count
            );
            if !result.validation_issues.is_empty() {
                for issue in &result.validation_issues {
                    eprintln!("validation issue: {issue}");
                }
                eprintln!(
                    "error: the emitted spec has {} validation issue(s)",
                    result.validation_issues.len()
                );
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

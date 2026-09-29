use clap::Parser;
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(about = "Generate native TypeScript bindings from an Open Graphene spec")]
struct Args {
    #[arg(long)]
    spec: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long)]
    check: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match open_graphene_gen_bindings_ts::generate(&args.spec, &args.out_dir, args.check) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

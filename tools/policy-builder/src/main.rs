//  MAIN.rs
//    by Lut99
//
//  Created:
//    29 Nov 2023, 15:11:08
//  Last edited:
//    29 Nov 2023, 17:34:18
//  Auto updated?
//    Yes
//
//  Description:
//!   Entrypoint for the `policy-builder` tool.
//

use std::collections::HashSet;
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::fs::{File, Permissions};
use std::io::{BufRead as _, BufReader, Read as _, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use clap::Parser;
use console::Style;
use error_trace::ErrorTrace as _;
use humanlog::{DebugMode, HumanLogger};
use log::{debug, error, info};
use policy_builder::compile::eflint_to_json;
use policy_builder::download::{download_file_async, DownloadSecurity};


/***** ARGUMENTS *****/
/// The arguments for the tool.
#[derive(Debug, Parser)]
struct Arguments {
    /// Whether to do INFO- and DEBUG-level statements.
    #[clap(long, global = true, help = "If given, enables INFO- and DEBUG-level log statements.")]
    debug: bool,
    /// Whether to do TRACE-level statements.
    #[clap(long, global = true, help = "If given, enables TRACE-level log statements. Implies '--debug'.")]
    trace: bool,

    /// The eFLINT file to compile.
    #[clap(name = "PATH", help = "Path pointing to the file to compile.")]
    path:   PathBuf,
    /// The file to compile to.
    #[clap(
        short,
        long,
        help = "If given, writes the result to a file at the given location instead of stdout. Use '-' to explicitly redirect to stdout."
    )]
    output: Option<String>,

    /// Overrides downloading to default location.
    #[clap(
        short,
        long,
        help = "If given, uses an existing 'eflint-to-json' executable. Otherwise, attempts to download from GitHub and writes to \
                `/tmp/eflint-to-json'."
    )]
    compiler: Option<PathBuf>,
}





/***** ENTRYPOINT *****/
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Parse the arguments
    let args: Arguments = Arguments::parse();

    // Setup the logger
    if let Err(err) = HumanLogger::terminal(DebugMode::from_flags(args.trace, args.debug)).init() {
        eprintln!("WARNING: Failed to setup logger: {err} (no logging for this session)");
    }
    info!("{} - v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    // Run the thing, then
    if let Err(err) = eflint_to_json(&args.path, args.output.as_ref(), args.compiler.as_ref()).await {}

    // Done
    println!(
        "Successfully compiled {} to {}",
        Style::new().bold().green().apply_to(args.path.display()),
        Style::new().bold().green().apply_to(output_path)
    );
}

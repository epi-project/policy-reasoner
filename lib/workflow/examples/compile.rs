//  COMPILE.rs
//    by Lut99
//
//  Created:
//    31 Oct 2023, 14:20:54
//  Last edited:
//    31 Oct 2023, 17:08:54
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a wrapper that converts the
//!   [WIR](brane_ast::ast::Workflow) into
//!   [checker workflows](brane_chk::workflow::spec::Workflow).
//

use std::fmt::{Display, Formatter, Result as FResult};
use std::path::PathBuf;
use std::str::FromStr;
use std::{error, fs};

use brane_ast::{ast, compile_program, CompileResult, ParserOptions};
use brane_chk::workflow::spec::Workflow;
use clap::Parser;
use enum_debug::EnumDebug;
use error_trace::ErrorTrace as _;
use humanlog::{DebugMode, HumanLogger};
use log::{debug, error, info, Level};
use specifications::data::DataIndex;
use specifications::package::PackageIndex;


/***** ERRORS *****/
/// Defines what may go wrong when parsing [`InputLanguage`]s from strings.
#[derive(Debug)]
enum InputLanguageParseError {
    /// The given string identifier was unknown.
    Unknown { raw: String },
}
impl Display for InputLanguageParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use InputLanguageParseError::*;
        match self {
            Unknown { raw } => write!(f, "Unknown input language '{raw}' (see `--help` for more information)"),
        }
    }
}
impl error::Error for InputLanguageParseError {}

/// Defines binary-level origin errors.
#[derive(Debug)]
enum Error {
    /// Failed to read an input file.
    InputRead { path: PathBuf, err: std::io::Error },
    /// Failed to deserialize the given file as a JSON WIR.
    InputDeserialize { path: PathBuf, err: serde_json::Error },
    /// Failed to compile the WIR into a Workflow
    InputCompile { path: PathBuf, err: brane_chk::workflow::compile::Error },
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Error::*;
        match self {
            InputRead { path, .. } => write!(f, "Failed to read input file '{}'", path.display()),
            InputDeserialize { path, .. } => write!(f, "Failed to deserialize input file '{}' as valid WIR JSON", path.display()),
            InputCompile { path, .. } => write!(f, "Failed to compile input file '{}' as a checker workflow", path.display()),
        }
    }
}
impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use Error::*;
        match self {
            InputRead { err, .. } => Some(err),
            InputDeserialize { err, .. } => Some(err),
            InputCompile { err, .. } => Some(err),
        }
    }
}





/***** ARGUMENTS *****/
/// Defines the possible input languages.
#[derive(Clone, Copy, Debug, EnumDebug, Eq, Hash, PartialEq)]
enum InputLanguage {
    /// BraneScript, which we turn into WIR before compiling.
    BraneScript,
    /// The WIR directory,
    WorkflowIntermediateRepresentation,
}
impl FromStr for InputLanguage {
    type Err = InputLanguageParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bs" | "bscript" | "branescript" => Ok(Self::BraneScript),
            "wir" => Ok(Self::WorkflowIntermediateRepresentation),
            raw => Err(InputLanguageParseError::Unknown { raw: raw.into() }),
        }
    }
}



/// Defines the toplevel arguments of the binary.
#[derive(Debug, Parser)]
struct Arguments {
    /// The path to the file to compile.
    #[clap(name = "INPUTS", help = "The input file(s) to compile. See `--language` to switch between what kind.")]
    inputs: Vec<PathBuf>,

    /// Enables debug.
    #[clap(long, global = true, help = "If given, enables INFO- and DEBUG-level log statements.")]
    debug: bool,
    /// Enables trace.
    #[clap(long, global = true, help = "If given, enables TRACE-level log statements. Implies '--debug'.")]
    trace: bool,
    /// The language of the input.
    #[clap(
        short,
        long,
        default_value = "bs",
        help = "The language of the input file(s). Options are: `bs`, `bscript` or `branescript` for BraneScript; and `wir` for the already \
                compiled WIR directly."
    )]
    language: InputLanguage,
    /// Whether to skip optimisation or not.
    #[clap(long, alias = "no-optimize", global = true, help = "If given, does not optimise the workflow before printing.")]
    no_optimise: bool,
}





/***** ENTRYPOINT *****/
fn main() {
    // Parse the arguments
    let args: Arguments = Arguments::parse();

    // Initialize the logger
    if let Err(err) = HumanLogger::terminal(DebugMode::from_flags(args.trace, args.debug)).init() {
        eprintln!("WARNING: Failed to setup logger: {err} (no logging for this session)");
    }
    info!("{} ({}) - v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    // Go thru the inputs
    for input in args.inputs {
        // Get the input file
        debug!("Reading file '{}' as {}...", input.display(), args.language.variant());
        let raw: String = match fs::read_to_string(&input) {
            Ok(raw) => raw,
            Err(err) => {
                error!("{}", Error::InputRead { path: input, err }.trace());
                std::process::exit(1);
            },
        };

        // If it's a BraneScript file, compile to WIR first
        let wir: ast::Workflow = if args.language == InputLanguage::BraneScript {
            // Compile to WIR first
            debug!("Compiling input {} to {}...", InputLanguage::BraneScript.variant(), InputLanguage::WorkflowIntermediateRepresentation.variant());
            match compile_program(raw.as_bytes(), &PackageIndex::empty(), &DataIndex::from_infos(vec![]).unwrap(), &ParserOptions::bscript()) {
                CompileResult::Workflow(wir, warns) => {
                    for warn in warns {
                        warn.prettyprint(input.display().to_string(), &raw);
                    }
                    wir
                },
                CompileResult::Unresolved(_, _) => {
                    unreachable!();
                },
                CompileResult::Program(_, _) => {
                    unreachable!();
                },

                CompileResult::Eof(eof) => {
                    eof.prettyprint(input.display().to_string(), raw);
                    error!("Compilation failed (see output above)");
                    std::process::exit(1);
                },
                CompileResult::Err(errs) => {
                    for err in errs {
                        err.prettyprint(input.display().to_string(), &raw);
                    }
                    error!("Compilation failed (see output above)");
                    std::process::exit(1);
                },
            }
        } else {
            // Simply deserialize the JSON
            match serde_json::from_str(&raw) {
                Ok(wir) => wir,
                Err(err) => {
                    error!("{}", Error::InputDeserialize { path: input, err }.trace());
                    std::process::exit(1);
                },
            }
        };

        // If debug, write the representation
        if log::max_level() >= Level::Debug {
            let mut debug_wir: Vec<u8> = Vec::new();
            brane_ast::traversals::print::ast::do_traversal(&wir, &mut debug_wir).unwrap();
            debug!("Input WIR:\n\n{}\n", String::from_utf8_lossy(&debug_wir));
        }

        // Now compile to our own representation!
        debug!("Compiling {} to CheckerWorkflow...", InputLanguage::WorkflowIntermediateRepresentation.variant());
        let mut wf: Workflow = match wir.try_into() {
            Ok(wf) => wf,
            Err(err) => {
                error!("{}", Error::InputCompile { path: input, err }.trace());
                std::process::exit(1);
            },
        };

        // Optimise if not told not to
        if !args.no_optimise {
            wf.optimize();
        }

        // That's it, print it
        println!("{}", wf.visualize());
    }
}

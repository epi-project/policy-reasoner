//  COMPILE.rs
//    by Lut99
//
//  Created:
//    31 Oct 2023, 14:20:54
//  Last edited:
//    29 Jan 2024, 15:57:57
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a wrapper that converts the
//!   [WIR](brane_ast::ast::Workflow) into
//!   [checker workflows](workflow::workflow::spec::Workflow).
//

use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FResult};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::{error, fs};

use brane_ast::{CompileResult, ParserOptions, ast, compile_program};
use brane_shr::utilities::{create_data_index_from, create_package_index_from};
use clap::Parser;
use enum_debug::EnumDebug;
use error_trace::ErrorTrace as _;
use humanlog::{DebugMode, HumanLogger};
use log::{Level, debug, error, info};
use specifications::data::DataIndex;
use specifications::package::PackageIndex;
use workflow::spec::Workflow;

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

/// Defines what may go wrong when parsing [`OutputLanguage`]s from strings.
#[derive(Debug)]
enum OutputLanguageParseError {
    /// The given string identifier was unknown.
    Unknown { raw: String },
}
impl Display for OutputLanguageParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use OutputLanguageParseError::*;
        match self {
            Unknown { raw } => write!(f, "Unknown output language '{raw}' (see `--help` for more information)"),
        }
    }
}
impl error::Error for OutputLanguageParseError {}

/// Defines binary-level origin errors.
#[derive(Debug)]
enum Error {
    /// Failed to read an input file.
    InputRead { path: PathBuf, err: std::io::Error },
    /// Failed to deserialize the given file as a JSON WIR.
    InputDeserialize { path: PathBuf, err: serde_json::Error },
    /// Failed to compile the WIR into a Workflow
    InputCompile { path: PathBuf, err: workflow::compile::Error },
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

/// Defines the possible output languages.
#[derive(Clone, Copy, Debug, EnumDebug, Eq, Hash, PartialEq)]
enum OutputLanguage {
    /// The workflow visualisation
    Workflow,
    /// JSON serialization
    Json,
    /// eFLINT JSON phrases
    #[cfg(feature = "eflint")]
    EFlintJson,
    /// eFLINT phrases
    #[cfg(feature = "eflint")]
    EFlint,
}
impl FromStr for OutputLanguage {
    type Err = OutputLanguageParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "wf" | "workflow" => Ok(Self::Workflow),
            "json" => Ok(Self::Json),
            #[cfg(feature = "eflint")]
            "eflint-json" => Ok(Self::EFlintJson),
            #[cfg(feature = "eflint")]
            "eflint" => Ok(Self::EFlint),
            raw => Err(OutputLanguageParseError::Unknown { raw: raw.into() }),
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
    input: InputLanguage,
    /// Language to write to
    #[clap(
        short,
        long,
        default_value = "workflow",
        help = "The language of the output. Options are: `wf` or `workflow` for the workflow visualisation; `json` for JSON; `eflint-json` for \
                eFLINT JSON; or 'eflint' for eFLINT phrases. Note that the latter two are only available when compiled with the `eflint`-feature."
    )]
    output: OutputLanguage,
    /// Whether to plan inputs.
    #[clap(long, help = "If given, plans tasks and input locations on the 'localhost' location.")]
    plan: bool,
    /// Whether to skip optimisation or not.
    #[clap(long, alias = "no-optimize", global = true, help = "If given, does not optimise the workflow before printing.")]
    no_optimise: bool,
    /// The location where to read packages from.
    #[clap(
        short,
        long,
        default_value = "./tests/packages",
        help = "The location where we're reading packages from to compile the test files. Ignored if input language is not BraneScript."
    )]
    packages_path: PathBuf,
    /// The location where to read datasets from.
    #[clap(
        short,
        long,
        default_value = "./tests/data",
        help = "The location where we're reading data from to compile the test files. Ignored if input language is not BraneScript."
    )]
    data_path: PathBuf,
    /// The user to add at the bottom of the workflow.
    #[clap(short, long, help = "If given, determines the name to add to the compiled workflow. Chooses a random one if omitted.")]
    user: Option<String>,
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
        debug!("Reading file '{}' as {}...", input.display(), args.input.variant());
        let raw: String = match fs::read_to_string(&input) {
            Ok(raw) => raw,
            Err(err) => {
                error!("{}", Error::InputRead { path: input, err }.trace());
                std::process::exit(1);
            },
        };

        // If it's a BraneScript file, compile to WIR first
        let wir: ast::Workflow = if args.input == InputLanguage::BraneScript {
            debug!("Input BraneScript:\n\n{raw}\n");

            // Get the package and data index
            debug!("Reading package index from '{}'...", args.packages_path.display());
            let pindex: PackageIndex = create_package_index_from(&args.packages_path);
            debug!("Reading data index from '{}'...", args.data_path.display());
            let dindex: DataIndex = create_data_index_from(&args.data_path);

            // Compile to WIR first
            debug!("Compiling input {} to {}...", InputLanguage::BraneScript.variant(), InputLanguage::WorkflowIntermediateRepresentation.variant());
            match compile_program(raw.as_bytes(), &pindex, &dindex, &ParserOptions::bscript()) {
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
            debug!("Input JSON:\n\n{raw}\n");
            match serde_json::from_str(&raw) {
                Ok(wir) => wir,
                Err(err) => {
                    error!("{}", Error::InputDeserialize { path: input, err }.trace());
                    std::process::exit(1);
                },
            }
        };

        // Plan using the dummy planner of Brane
        let mut wir: ast::Workflow = if args.plan { brane_exe::dummy::DummyPlanner::plan(&mut HashMap::new(), wir) } else { wir };
        // Also assign a dummy user if not given
        wir.user = Arc::new(Some(args.user.clone().unwrap_or_else(|| names::three::usualcase::rand().into())));

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
        match args.output {
            OutputLanguage::Workflow => println!("{}", wf.visualize()),
            OutputLanguage::Json => match serde_json::to_string_pretty(&wf) {
                Ok(wf) => println!("{wf}"),
                Err(err) => {
                    error!("{}", err.trace());
                    std::process::exit(1);
                },
            },
            #[cfg(feature = "eflint")]
            OutputLanguage::EFlintJson => match serde_json::to_string_pretty(&wf.to_eflint()) {
                Ok(phrases) => println!("{phrases}"),
                Err(err) => {
                    error!("{}", err.trace());
                    std::process::exit(1);
                },
            },
            #[cfg(feature = "eflint")]
            OutputLanguage::EFlint => {
                for phrase in wf.to_eflint() {
                    print!("{:#}", <eflint_json::spec::Phrase as eflint_json::DisplayEFlint>::display_syntax(&phrase));
                }
            },
        }
    }
}

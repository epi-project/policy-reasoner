//  COMPILE.rs
//    by Lut99
//
//  Created:
//    29 Nov 2023, 17:22:57
//  Last edited:
//    29 Nov 2023, 17:30:47
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines functions for working with Olaf's `eflint-to-json` compiler.
//

use std::borrow::Cow;
use std::collections::HashSet;
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::fs::{File, Permissions};
use std::io::{BufRead as _, BufReader, Read as _, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use console::Style;
use error_trace::ErrorTrace as _;
use log::{debug, error};

use crate::download::{download_file_async, DownloadSecurity};


/***** CONSTANTS *****/
/// Compiler download URL.
const COMPILER_URL: &str = "https://github.com/Olaf-Erkemeij/eflint-server/raw/bd3997df89441f13cbc82bd114223646df41540d/eflint-to-json";
/// Compiler download checksum.
const COMPILER_CHECKSUM: [u8; 32] = hex_literal::hex!("4e4e59b158ca31e532ec0a22079951788696ffa5d020b36790b4461dbadec83d");





/***** ERRORS *****/
/// Defines toplevel errors.
#[derive(Debug)]
pub enum Error {
    /// Failed to read from child stdout.
    ChildRead { err: std::io::Error },
    /// Failed to write to child stdin.
    ChildWrite { err: std::io::Error },
    /// Failed to download the compiler.
    CompilerDownload { from: String, to: PathBuf, err: crate::download::Error },
    /// Failed to create the output file.
    FileCreate { path: PathBuf, err: std::io::Error },
    /// Failed to get metadata of file.
    FileMetadata { path: PathBuf, err: std::io::Error },
    /// Failed to open the input file.
    FileOpen { path: PathBuf, err: std::io::Error },
    /// Failed to set permissions of file.
    FilePermissions { path: PathBuf, err: std::io::Error },
    /// Failed to read the input file.
    FileRead { path: PathBuf, err: std::io::Error },
    /// Failed to write to the output file.
    FileWrite { path: String, err: std::io::Error },
    /// Failed to open included file.
    IncludeOpen { parent: PathBuf, path: PathBuf, err: std::io::Error },
    /// Missing a quote in the `#include`-string.
    MissingQuote { parent: PathBuf, raw: String },
    /// Failed to canonicalize the given path.
    PathCanonicalize { parent: PathBuf, path: PathBuf, err: std::io::Error },
    /// Failed to spawn the eflint-to-json compiler process.
    Spawn { cmd: Command, err: std::io::Error },
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Error::*;
        match self {
            ChildRead { .. } => write!(f, "Failed to read from child stdin"),
            ChildWrite { .. } => write!(f, "Failed to write to child stdin"),
            CompilerDownload { from, to, .. } => write!(f, "Failed to download 'eflint-to-json' compiler from '{}' to '{}'", from, to.display()),
            FileCreate { path, .. } => write!(f, "Failed to create output file '{}'", path.display()),
            FileMetadata { path, .. } => write!(f, "Failed to get metadata of file '{}'", path.display()),
            FileOpen { path, .. } => write!(f, "Failed to open input file '{}'", path.display()),
            FilePermissions { path, .. } => write!(f, "Failed to set permissions of file '{}'", path.display()),
            FileRead { path, .. } => write!(f, "Failed to read from input file '{}'", path.display()),
            FileWrite { path, .. } => write!(f, "Failed to write to output file '{path}'"),
            IncludeOpen { parent, path, .. } => write!(f, "Failed to open included file '{}' (in file '{}')", path.display(), parent.display()),
            MissingQuote { parent, raw } => write!(f, "Missing quotes (\") in '{}' (in file '{}')", raw, parent.display()),
            PathCanonicalize { parent, path, .. } => write!(f, "Failed to canonicalize path '{}' (in file '{}')", path.display(), parent.display()),
            Spawn { cmd, .. } => write!(f, "Failed to spawn command {cmd:?}"),
        }
    }
}
impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use Error::*;
        match self {
            ChildRead { err, .. } => Some(err),
            ChildWrite { err, .. } => Some(err),
            CompilerDownload { err, .. } => Some(err),
            FileCreate { err, .. } => Some(err),
            FileMetadata { err, .. } => Some(err),
            FileOpen { err, .. } => Some(err),
            FilePermissions { err, .. } => Some(err),
            FileRead { err, .. } => Some(err),
            FileWrite { err, .. } => Some(err),
            IncludeOpen { err, .. } => Some(err),
            MissingQuote { .. } => None,
            PathCanonicalize { err, .. } => Some(err),
            Spawn { err, .. } => Some(err),
        }
    }
}





/***** HELPER FUNCTIONS *****/
/// Analyses a potential `#input(...)` or `#require(...)` line from eFLINT.
///
/// # Arguments
/// - `imported`: The set of already imported files (relevant for require).
/// - `path`: The path of the current file.
/// - `line`: The parsed line.
///
/// # Returns
/// A handle to the included file (as a tuple of the path + the handle) if any, or else [`None`].
///
/// # Errors
/// This function can error if we failed to open the included file.
fn potentially_include(imported: &mut HashSet<PathBuf>, path: &Path, line: &str) -> Result<Option<Option<(PathBuf, File)>>, Error> {
    // Strip whitespace
    let line: &str = line.trim();

    // Check it's a line
    if (line.len() >= 8 && &line[..8] == "#include") || (line.len() >= 8 && &line[..8] == "#require") {
        return Ok(None);
    }

    // Extract the text
    let squote: usize = match line.find('"') {
        Some(pos) => pos,
        None => return Err(Error::MissingQuote { parent: path.into(), raw: line.into() }),
    };
    let equote: usize = match line.rfind('"') {
        Some(pos) => pos,
        None => return Err(Error::MissingQuote { parent: path.into(), raw: line.into() }),
    };
    let incl_path: &str = &line[squote + 1..equote];

    // Build the path
    let incl_path: PathBuf = incl_path.into();
    let incl_path: PathBuf = if incl_path.is_absolute() { incl_path } else { path.join(incl_path) };
    let incl_path: PathBuf = match incl_path.canonicalize() {
        Ok(path) => path,
        Err(err) => return Err(Error::PathCanonicalize { parent: path.into(), path: incl_path, err }),
    };

    // Check if we've seen this before if it's require
    if &line[..8] == "#require" && imported.contains(&incl_path) {
        return Ok(Some(None));
    }

    // Build the path and attempt to open it
    let handle: File = match File::open(&incl_path) {
        Ok(handle) => handle,
        Err(err) => return Err(Error::IncludeOpen { parent: path.into(), path: incl_path, err }),
    };

    // OK
    Ok(Some(Some((incl_path, handle))))
}

/// Streams the given file's contents to the stdin of the given process, including files as necessary halfway.
///
/// # Arguments
fn load_input(imported: &mut HashSet<PathBuf>, path: &Path, handle: BufReader<File>, child: &mut ChildStdin) -> Result<(), Error> {
    debug!("Importing file '{}'", path.display());

    // Read the lines for the file
    for line in handle.lines() {
        // Unwrap the line
        let line: String = match line {
            Ok(line) => line,
            Err(err) => return Err(Error::FileRead { path: path.into(), err }),
        };

        // See if a file is included
        match potentially_include(imported, path, &line)? {
            Some(Some((child_path, child_handle))) => {
                load_input(imported, &child_path, BufReader::new(child_handle), child)?;
            },
            // We don't want to write the line since we already imported it
            Some(None) => {},
            None => {
                if let Err(err) = child.write_all(line.as_bytes()) {
                    return Err(Error::ChildWrite { err });
                }
                if let Err(err) = child.write_all(b"\n") {
                    return Err(Error::ChildWrite { err });
                }
            },
        }
    }

    // Done!
    Ok(())
}




/***** LIBRARY *****/
/// Compiles a (tree of) `.eflint` files using Olaf's `eflint-to-json` compiler.
///
/// Resolves relative paths in the files as relative to the file in which they occur.
///
/// # Arguments
/// - `input`: The input file to compile. Any `#include`s and `#require`s will be handled, building a tree of files to import.
/// - `output`: Where to write the output to. Omit to use stdout.
/// - `compiler`: If given, will not download a compiler to `/tmp/eflint-to-json` but will instead use the given one.
///
/// # Errors
/// This function may error for a plethora of reasons.
pub async fn eflint_to_json(input_path: &Path, output_path: Option<&Path>, compiler_path: Option<&Path>) -> Result<(), Error> {
    // Resolve the compiler
    let compiler_path: Cow<Path> = match compiler_path {
        Some(path) => Cow::Borrowed(path),
        None => {
            // Get the output path
            let compiler_path: PathBuf = std::env::temp_dir().join("eflint-to-json");

            // Download it if it does not exist (or at least, give it a try)
            if !compiler_path.exists() {
                // Download the file...
                if let Err(err) = download_file_async(
                    COMPILER_URL,
                    &compiler_path,
                    DownloadSecurity { checksum: Some(&COMPILER_CHECKSUM), https: true },
                    Some(Style::new().bold().green()),
                )
                .await
                {
                    error!("{}", Error::CompilerDownload { from: COMPILER_URL.into(), to: compiler_path, err }.trace());
                    std::process::exit(1);
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;

                    // ...and make it executable
                    let mut perms: Permissions = match std::fs::metadata(&compiler_path) {
                        Ok(mdata) => mdata.permissions(),
                        Err(err) => {
                            error!("{}", Error::FileMetadata { path: compiler_path, err }.trace());
                            std::process::exit(1);
                        },
                    };
                    perms.set_mode(perms.mode() | 0o500);
                    if let Err(err) = std::fs::set_permissions(&compiler_path, perms) {
                        error!("{}", Error::FilePermissions { path: compiler_path, err }.trace());
                        std::process::exit(1);
                    }
                }
            }

            // Return the path
            Cow::Owned(compiler_path)
        },
    };
    debug!("Using compiler at: '{}'", compiler_path.display());

    // Open the input file
    debug!("Opening input file '{}'", input_path.display());
    let input: File = match File::open(input_path) {
        Ok(input) => input,
        Err(err) => {
            error!("{}", Error::FileOpen { path: input_path.into(), err }.trace());
            std::process::exit(1);
        },
    };

    // Open the output file
    let (mut output, output_path): (Box<dyn Write>, Cow<str>) = match output_path {
        Some(path) => match File::create(path) {
            Ok(input) => (Box::new(input), path.to_string_lossy()),
            Err(err) => {
                error!("{}", Error::FileCreate { path: path.into(), err }.trace());
                std::process::exit(1);
            },
        },
        None => (Box::new(std::io::stdout()), "<stdout>".into()),
    };

    // Alrighty well open a handle to the compiler
    debug!("Spawning compiler '{}'", compiler_path.display());
    let mut cmd: Command = Command::new(compiler_path.to_string_lossy().as_ref());
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut handle: Child = match cmd.spawn() {
        Ok(handle) => handle,
        Err(err) => {
            error!("{}", Error::Spawn { cmd, err }.trace());
            std::process::exit(1);
        },
    };

    // Feed the input to the compiler, analyzing for `#input(...)` and `#require(...)`
    debug!("Reading input to child process...");
    let mut stdin: ChildStdin = handle.stdin.take().unwrap();
    let mut included: HashSet<PathBuf> = HashSet::new();
    if let Err(err) = load_input(&mut included, input_path, BufReader::new(input), &mut stdin) {
        error!("{}", err.trace());
        std::process::exit(1);
    }
    drop(stdin);

    // Alrighty, now it's time to stream the output of the child to the output file
    debug!("Writing child process output to '{output_path}'...");
    let mut chunk: [u8; 65535] = [0; 65535];
    let mut stdout: ChildStdout = handle.stdout.take().unwrap();
    loop {
        // Read the next chunk
        let chunk_len: usize = match stdout.read(&mut chunk) {
            Ok(len) => len,
            Err(err) => {
                error!("{}", Error::ChildRead { err }.trace());
                std::process::exit(1);
            },
        };
        if chunk_len == 0 {
            break;
        }

        // Write to the file
        if let Err(err) = output.write_all(&chunk) {
            error!("{}", Error::FileWrite { path: output_path.into(), err }.trace());
            std::process::exit(1);
        }
    }

    // Done
    Ok(())
}

//  BUILD.rs
//    by Lut99
//
//  Created:
//    13 Dec 2023, 11:45:11
//  Last edited:
//    15 Dec 2023, 14:43:14
//  Auto updated?
//    Yes
//
//  Description:
//!   Build script for the main `policy-reasoner` executable.
//!
//!   In particular, charged with compiling the eFLINT interface to eFLINT JSON before it can be included in the executable.
//

use std::env::VarError;
use std::fmt::{Display, Formatter, Result as FResult};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{env, error};

use eflint_to_json::compile;
use error_trace::ErrorTrace as _;
use sha2::{Digest as _, Sha256};


/***** ERRORS *****/
/// Defines errors originating from the buildscript
#[derive(Debug)]
enum Error {
    /// Failed to get some environment variable.
    EnvRetrieve { name: &'static str, err: std::env::VarError },
    /// Failed to compile the input
    InputCompile { path: PathBuf, err: eflint_to_json::Error },
    /// Failed to create the output file
    OutputCreate { path: PathBuf, err: std::io::Error },
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Error::*;
        match self {
            EnvRetrieve { name, .. } => write!(f, "Failed to get environment variable '{name}'"),
            InputCompile { path, .. } => write!(f, "Failed to compile input file '{}'", path.display()),
            OutputCreate { path, .. } => write!(f, "Failed to create output file '{}'", path.display()),
        }
    }
}
impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use Error::*;
        match self {
            EnvRetrieve { err, .. } => Some(err),
            InputCompile { err, .. } => Some(err),
            OutputCreate { err, .. } => Some(err),
        }
    }
}





/***** HELPERS *****/
/// Wraps around another Writer to always Write while updating a hash of whatever we write.
struct HashWriter<W>(W, Sha256);
impl<W> HashWriter<W> {
    /// Constructor for the HashWriter that initializes its digest.
    ///
    /// # Arguments
    /// - `writer`: The [`Write`]r to wrap.
    ///
    /// # Returns
    /// A new instance of a HashWriter.
    #[inline]
    fn new(writer: W) -> Self { Self(writer, Sha256::new()) }

    /// Finalizes the HashWriter and returns the digest.
    ///
    /// # Returns
    /// The raw digest bytes encoded as Base64 (in constant time yay).
    #[inline]
    fn finalize(self) -> String { base16ct::lower::encode_string(&self.1.finalize()) }
}
impl<W: Write> Write for HashWriter<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Update the hasher first before passing to the wrapper impl
        self.1.update(buf);
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        // Flush only
        self.0.flush()
    }
}





/***** ENTRYPOINT *****/
fn main() {
    // Read some environment variables
    let src_dir: PathBuf = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let eflint_to_json_exe: Option<PathBuf> = match env::var("EFLINT_TO_JSON_PATH") {
        Ok(path) => {
            let path: PathBuf = path.into();
            if path.is_relative() { Some(src_dir.join(path)) } else { Some(path) }
        },
        Err(VarError::NotPresent) => None,
        Err(err) => panic!("{}", Error::EnvRetrieve { name: "EFLINT_TO_JSON_PATH", err }.trace()),
    };

    // Mark the input files as source-dependent
    let interface_dir: PathBuf = src_dir.join("policy").join("eflint").join("interface");
    println!("cargo:rerun-if-changed={}", interface_dir.display());
    println!("cargo:rerun-if-env-changed=EFLINT_TO_JSON_PATH");

    // Compute the concrete input- and output paths
    let main_path: PathBuf = interface_dir.join("main.eflint");
    let output_file: PathBuf = PathBuf::from(env::var("OUT_DIR").unwrap()).join("base-defs.eflint.json");
    println!("cargo:rustc-env=BASE_DEFS_EFLINT_JSON={}", output_file.display());

    // Alright attempt to open the output file
    let handle: File = match File::create(&output_file) {
        Ok(handle) => handle,
        Err(err) => panic!("{}", Error::OutputCreate { path: output_file, err }.trace()),
    };
    let mut handle: HashWriter<File> = HashWriter::new(handle);

    // Alright run the compiler, after which we reset the handle
    if let Err(err) = compile(&main_path, &mut handle, eflint_to_json_exe.as_ref().map(|p| p.as_path())) {
        panic!("{}", Error::InputCompile { path: main_path, err }.trace());
    }

    // Also set the found hash
    let hash: String = handle.finalize();
    println!("cargo:rustc-env=BASE_DEFS_EFLINT_JSON_HASH={hash}");

    // Done
}

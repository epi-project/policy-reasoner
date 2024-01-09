//  STATE.rs
//    by Lut99
//
//  Created:
//    09 Jan 2024, 13:14:34
//  Last edited:
//    09 Jan 2024, 13:43:52
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements resolvers for the policy state, e.g., which datasets
//!   there are, which domains, etc.
//

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use state_resolver::{State, StateResolver};


/***** ERRORS *****/
/// Defines errors occurring in the [`FileStateResolver`].
#[derive(Debug)]
pub enum FileStateResolverError {
    /// Failed to read a file.
    FileRead { path: PathBuf, err: std::io::Error },
    /// Failed to deserialize a file into JSON.
    FileDeserialize { path: PathBuf, err: serde_json::Error },
}
impl Display for FileStateResolverError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use FileStateResolverError::*;
        match self {
            FileRead { path, .. } => write!(f, "Failed to read file '{}'", path.display()),
            FileDeserialize { path, .. } => write!(f, "Failed to deserialize file '{}' as JSON", path.display()),
        }
    }
}
impl Error for FileStateResolverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use FileStateResolverError::*;
        match self {
            FileRead { err, .. } => Some(err),
            FileDeserialize { err, .. } => Some(err),
        }
    }
}



/// Defines errors occurring in the [`BraneApiResolver`].
#[derive(Debug)]
pub enum BraneApiResolverError {}
impl Display for BraneApiResolverError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        // use BraneApiResolverError::*;
        // match self {

        // }
        Ok(())
    }
}
impl Error for BraneApiResolverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // use BraneApiResolverError::*;
        // match self {

        // }
        None
    }
}





/***** LIBRARY *****/
/// Defines a resolver that resolves from a static file.
#[derive(Debug)]
pub struct FileStateResolver {
    /// The state read from the file.
    state: State,
}

impl FileStateResolver {
    /// Constructor for the FileStateResolver.
    ///
    /// # Arguments
    /// - `path`: The path of the file to use for resolving.
    ///
    /// # Returns
    /// A new FileStateResolver instance.
    ///
    /// # Errors
    /// This function may error if it failed to read the given file.
    #[inline]
    pub fn new(path: impl AsRef<Path>) -> Result<Self, FileStateResolverError> {
        let path: &Path = path.as_ref();

        // Read the file in one go
        let state: String = match fs::read_to_string(&path) {
            Ok(state) => state,
            Err(err) => return Err(FileStateResolverError::FileRead { path: path.into(), err }),
        };

        // Parse it as JSON
        let state: State = match serde_json::from_str(&state) {
            Ok(state) => state,
            Err(err) => return Err(FileStateResolverError::FileDeserialize { path: path.into(), err }),
        };

        // Build ourselves with it
        Ok(Self { state })
    }
}

#[async_trait]
impl StateResolver for FileStateResolver {
    type Error = std::convert::Infallible;

    async fn get_state(&self) -> Result<State, Self::Error> {
        // Simply return a clone of the internal one
        Ok(self.state.clone())
    }
}



/// Defines a resolver that resolves state using Brane's API service.
#[derive(Debug)]
pub struct BraneApiResolver {}

#[async_trait]
impl StateResolver for BraneApiResolver {
    type Error = BraneApiResolverError;

    async fn get_state(&self) -> Result<State, Self::Error> { Ok(State { users: vec![], locations: vec![], datasets: vec![], functions: vec![] }) }
}

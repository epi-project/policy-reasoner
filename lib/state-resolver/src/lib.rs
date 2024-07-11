use std::error::Error;

use serde::{Deserialize, Serialize};
use workflow::spec::{Dataset, User};

/***** ERRORS *****/
/// Defines some errors being constructable in the type used in the [`StateResolver`].
pub trait StateResolverError {
    /// Checks if this error was generated because the `use_case` identifier supplied to [`StateResolver::get_state()`] was not recognized.
    ///
    /// # Returns
    /// The given use_case identifier as a [`String`], or [`None`] if this error does not represent this case.
    fn try_as_unknown_use_case(&self) -> Option<&String>;
}

/// We implement it for `std::convert::Infallible` to allow implementations to not care about errors.
impl StateResolverError for std::convert::Infallible {
    #[inline]
    fn try_as_unknown_use_case(&self) -> Option<&String> {
        // It will never error, so it can never be an unknown case
        None
    }
}

/***** AUXILLARY *****/
/// The state that captures runtime context, returned by a [`StateResolver`] dynamically.
///
/// This defines everything a policy gets to know about the state of the system at the time a policy is being checked.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    // Only scientists for now
    pub users:     Vec<User>,
    pub locations: Vec<User>,
    pub datasets:  Vec<Dataset>,
    pub functions: Vec<Dataset>,
    // TODO: Somehow add events / audit trail
    // TODO: Somehow add duties or duty policies, maybe encode in Dataset?
}

/***** LIBRARY *****/
/// Defines how a state resolver looks like in general.
#[async_trait::async_trait]
pub trait StateResolver {
    /// The error type emitted by this trait's functions.
    ///
    /// Note that the error is supposed to implement [`StateResolverError`] to communicate standard errors.
    type Error: 'static + Send + StateResolverError + Sync + Error;

    /// Retrieves the current reasoner state necessary for resolving policies.
    ///
    /// Note that this state is agnostic to the specific reasoner connector used (and therefore policy language).
    ///
    /// # Arguments
    /// - `use_case`: Some identifier that allows the state resolver to assume a different state depending on the use-case used.
    ///
    /// # Returns
    /// A new [`State`] struct that encodes the current state.
    ///
    /// # Errors
    /// This function may error whenever it likes. However, it's recommended to trigger the errors specified in the [`StateResolverError`] trait if applicable.
    async fn get_state(&self, use_case: String) -> Result<State, Self::Error>;
}

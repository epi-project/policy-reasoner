//  SPEC.rs
//    by Lut99
//
//  Created:
//    18 Jan 2024, 16:06:11
//  Last edited:
//    18 Jan 2024, 16:13:31
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the interface for the implementations provided in this
//!   crate.
//

use std::error::Error;


/***** LIBRARY *****/
/// Defines that a particular type can parse nested CLI commands.
///
/// The commands should be given as a string, which the parser then extracts as arguments.
pub trait NestedCliParser {
    /// The associated state that is parsed from the nested argument list.
    type Args;
    /// Any error that is thrown when parsing.
    type ParseError: Error;


    /// Parses the given string as a set of nested CLI arguments.
    ///
    /// # Arguments
    /// - `args`: The raw [`&str`] to parse.
    ///
    /// # Returns
    /// A new instance of [`Self::Arguments`](NestedCliParser::Arguments) that contains the information parsed from the raw `args`.
    ///
    /// # Errors
    /// This function has free roam to error as it desires. Typically, however, this should be when the input is invalid for this parser.
    fn parse(&self, args: &str) -> Result<Self::Args, Self::ParseError>;
}

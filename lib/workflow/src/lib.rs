//  LIB.rs
//    by Lut99
//
//  Created:
//    27 Oct 2023, 15:54:17
//  Last edited:
//    08 Nov 2023, 10:18:46
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the workflow representation used internally by the checker.
//

// Declare the subsubmodules
pub mod compile;
pub mod optimize;
pub mod preprocess;
pub mod spec;
#[cfg(test)]
pub mod tests;
pub mod utils;
pub mod visualize;


/***** CONSTANTS *****/
/// Defines the location of the tests
#[cfg(test)]
const TESTS_DIR: &str = "../../tests";

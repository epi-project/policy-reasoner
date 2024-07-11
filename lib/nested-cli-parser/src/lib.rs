//  LIB.rs
//    by Lut99
//
//  Created:
//    18 Jan 2024, 16:05:48
//  Last edited:
//    19 Jan 2024, 14:50:14
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines an interface (and some implementations) for defining CLI
//!   parsing for plugins by processing nested commands.
//

// Declare nested modules
#[cfg(feature = "map_parser")]
pub mod map_parser;
pub mod spec;

// Bring some of it into the main namespace
pub use spec::*;

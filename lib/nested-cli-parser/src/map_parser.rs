//  MAP PARSER.rs
//    by Lut99
//
//  Created:
//    18 Jan 2024, 16:07:04
//  Last edited:
//    29 Jan 2024, 16:02:40
//  Auto updated?
//    Yes
//
//  Description:
//!   Common implementation of a nested parser that simply parses a list
//!   of key/value pairs.
//

use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};

use unicode_segmentation::UnicodeSegmentation;

use crate::NestedCliParser;


/***** HELPER MACROS *****/
/// Given a character, determines if it's valid for a key identifier.
macro_rules! is_valid_key_char {
    ($c:ident) => {{
        if !$c.is_ascii() || $c.len() != 1 {
            false
        } else {
            let c: char = $c.chars().next().unwrap();
            ((c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '-' || c == '_')
        }
    }};
}





/***** ERRORS *****/
/// Defines errors that may originate from the [`MapParser`].
#[derive(Debug)]
pub enum Error {
    /// An argument contained an unescaped `=` twice.
    DuplicateEquals { prev_pos: usize, pos: usize },
    /// An equals sign was found without an actual value.
    EmptyValue { pos: usize },
    /// A key contained an illegal character for a key.
    IllegalKeyChar { pos: usize, c: String },
    /// A key was encountered that wasn't known.
    UnknownKey { pos: usize, key: String },
    /// An escape character was found without a matching next character.
    UnmatchedEscape { esc: usize, pos: usize },
    /// A quote was found without a terminating counterpart.
    UnmatchedQuote { first: usize, pos: usize },
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Error::*;
        match self {
            DuplicateEquals { prev_pos, pos } => {
                write!(f, "Encountered a second unescaped equals sign '=' at character {pos} (first equals sign was at {prev_pos})")
            },
            EmptyValue { pos } => write!(f, "Expected value for key after equals sign '=' at character {pos}"),
            IllegalKeyChar { pos, c } => write!(f, "Encountered illegal character for a key {c:?} at position {pos}"),
            UnknownKey { pos, key } => write!(f, "Unknown option '{key}' at character {pos}"),
            UnmatchedEscape { esc, pos } => write!(f, "Expected escaped character at position {pos} (to follow up escape character '/' at {esc})"),
            UnmatchedQuote { first, pos } => {
                write!(f, "Expected terminating quote '\"' at position {pos} (to close quote character '\"' at {first})")
            },
        }
    }
}
impl error::Error for Error {}





/***** HELPER FUNCTIONS *****/
/// Parses a single key/value argument.
///
/// # Arguments
/// - `keys`: A list of known keys that can we accept.
/// - `arg`: The buffer containing the single argument.
/// - `arg_pos`: The position of the argument within the entire input.
///
/// # Returns
/// A pair of the parsed key and an optional value if the user gave one.
///
/// # Errors
/// This function errors if the input was not a valid key/value pair.
fn parse_arg(keys: &HashSet<String>, arg: &str, arg_pos: usize) -> Result<(String, Option<String>), Error> {
    // Go through the buffer to find the equals character in a similar fashion
    let mut key: Option<String> = None;
    let mut mode: ArgParseMode = ArgParseMode::Key;
    let mut buf: String = String::new();
    for (pos, c) in arg.grapheme_indices(true) {
        match &mode {
            ArgParseMode::Key => match c {
                // Equals is how we recognize we've seen the key
                "=" => {
                    // The buffer is now only valid alphanumerical values
                    key = Some(buf);
                    buf = String::new();

                    // Move to the value
                    mode = ArgParseMode::Value(pos);
                },

                // Only accept key characters tho
                c if is_valid_key_char!(c) => buf.push_str(c),
                c => return Err(Error::IllegalKeyChar { pos: arg_pos + pos, c: c.into() }),
            },
            ArgParseMode::Value(eq_pos) => match c {
                // For values, we only focus on escapes
                "\"" => mode = ArgParseMode::Quotes(pos, Box::new(mode)),
                "\\" => mode = ArgParseMode::Escaped(pos, Box::new(mode)),

                // The rest is all valid - except equals
                "=" => return Err(Error::DuplicateEquals { prev_pos: *eq_pos, pos: arg_pos + pos }),
                c => buf.push_str(c),
            },

            ArgParseMode::Quotes(_, prev_mode) => match c {
                // We end quote mode if we see a quote :)
                "\"" => mode = (**prev_mode).clone(),

                // Escape on the escape character
                "\\" => mode = ArgParseMode::Escaped(pos, Box::new(mode)),

                // The rest is still valid
                c => buf.push_str(c),
            },
            ArgParseMode::Escaped(_, prev_mode) => match c {
                // A list of special whitespace characters
                "n" => {
                    buf.push_str("\n");
                    mode = (**prev_mode).clone();
                },
                "t" => {
                    buf.push_str("\t");
                    mode = (**prev_mode).clone();
                },
                "0" => {
                    buf.push_str("\0");
                    mode = (**prev_mode).clone();
                },

                // The rest is passed literally (including quotes and escape characters itself)
                c => {
                    buf.push_str(c);
                    mode = (**prev_mode).clone();
                },
            },
        }
    }

    // Assert nothing is left unmatched
    match mode {
        ArgParseMode::Quotes(first_pos, _) => return Err(Error::UnmatchedQuote { first: arg_pos + first_pos, pos: arg_pos + buf.len() }),
        ArgParseMode::Escaped(esc_pos, _) => return Err(Error::UnmatchedEscape { esc: arg_pos + esc_pos, pos: arg_pos + buf.len() }),
        ArgParseMode::Key | ArgParseMode::Value(_) => {},
    }

    // Resolve the remaining buffer
    let (key, value): (String, Option<String>) = if key.is_none() {
        (buf, None)
    } else if !buf.is_empty() {
        (key.unwrap(), Some(buf))
    } else {
        return Err(Error::EmptyValue { pos: arg_pos + buf.len() });
    };

    // Assert the key checks out
    if !keys.contains(&key) {
        return Err(Error::UnknownKey { pos: arg_pos, key });
    }

    // Alright return them
    Ok((key, value))
}





/***** HELPERS *****/
/// Defines possible modes of parsing the entire CLI string.
#[derive(Clone, Debug)]
enum ParseMode {
    /// The start mode, where we assume a raw string unless we see quotes
    Start,
    /// The quotes mode, where we've seen an outer quote.
    ///
    /// The position of the first quote is stored.
    Quotes(usize),
    /// We've entered an escaped character.
    ///
    /// The position of the backslash (`\`) is stored, together with the previous state.
    Escaped(usize, Box<Self>),
}

/// Defines possible modes of parsing a single key/value pair.
#[derive(Clone, Debug)]
enum ArgParseMode {
    /// The start mode, where we're parsing keys
    Key,
    /// The default mode for parsing values.
    ///
    /// The position of the separating equals is stored.
    Value(usize),
    /// The quotes mode, where we've seen an outer quote.
    ///
    /// The position of the first quote is stored, together with the previous state.
    Quotes(usize, Box<Self>),
    /// We've entered an escaped character.
    ///
    /// The position of the backslash (`\`) is stored, together with the previous state.
    Escaped(usize, Box<Self>),
}





/***** LIBRARY *****/
/// Common implementation of a nested parser that simply parses a list of key/value pairs.
#[derive(Debug)]
pub struct MapParser {
    /// The list of keys that are recognized by this parser.
    pub keys: HashSet<String>,
}
impl MapParser {
    /// Constructor for the MapParser.
    ///
    /// # Arguments
    /// - `keys`: A set of keys that are recognized by the parser. All others will eventually trigger [`UnknownKey`](Error::UnknownKey) errors down the line.
    ///
    /// # Returns
    /// A new [`MapParser`] instance.
    ///
    /// # Panics
    /// This function panics if any of the keys are not simple alphanumber strings (only underscores and dashes are allowed).
    pub fn new<S: AsRef<str> + Into<String>>(keys: impl IntoIterator<Item = S>) -> Self {
        // Build the set of keys
        let iter = keys.into_iter();
        let (min, max): (usize, Option<usize>) = iter.size_hint();
        let mut keys: HashSet<String> = HashSet::with_capacity(if let Some(max) = max { max } else { min });
        for key in iter {
            let rkey: &str = key.as_ref();
            // Assert it only exists of valid characters
            for (i, c) in rkey.grapheme_indices(true) {
                if !is_valid_key_char!(c) {
                    panic!("Given key '{rkey}' has illegal character '{c}' at index {i} (only alphanumeric characters, '-' and '_' are allowed)");
                }
            }

            // Add it if it passes
            keys.insert(key.into());
        }

        // OK, build self
        Self { keys }
    }
}
impl NestedCliParser for MapParser {
    type Args = HashMap<String, Option<String>>;
    type ParseError = Error;

    fn parse(&self, args: &str) -> Result<Self::Args, Self::ParseError> {
        // Parse the arguments using a little state machine to be respectful to quotes
        let mut parsed_args: HashMap<String, Option<String>> = HashMap::with_capacity(args.chars().filter(|c| *c == ',').count());
        let mut mode: ParseMode = ParseMode::Start;
        let mut buf: String = String::new();
        for (pos, c) in args.grapheme_indices(true) {
            match mode {
                // Simply parse the contents until we discover a comma
                ParseMode::Start => match c {
                    // Comma indicate the end of one arguments
                    "," => {
                        // `buf` now contains the entire argument, so parse it as such
                        let (key, value): (String, Option<String>) = parse_arg(&self.keys, &buf, pos)?;

                        // Alright, add the key/value pair!
                        parsed_args.insert(key, value);
                    },

                    // Mode changers
                    "\"" => mode = ParseMode::Quotes(pos),
                    "\\" => mode = ParseMode::Escaped(pos, Box::new(mode)),

                    // Default; simply accept into the buffer
                    c => buf.push_str(c),
                },

                ParseMode::Quotes(_) => match c {
                    // We end quote mode if we see a quote :)
                    "\"" => mode = ParseMode::Start,

                    // Escape on the escape character
                    "\\" => mode = ParseMode::Escaped(pos, Box::new(mode)),

                    // The rest is still valid
                    c => buf.push_str(c),
                },
                ParseMode::Escaped(_, prev_mode) => match c {
                    // A list of special whitespace characters
                    "n" => {
                        buf.push_str("\n");
                        mode = *prev_mode;
                    },
                    "t" => {
                        buf.push_str("\t");
                        mode = *prev_mode;
                    },
                    "0" => {
                        buf.push_str("\0");
                        mode = *prev_mode;
                    },

                    // The rest is passed literally (including quotes and escape characters itself)
                    c => {
                        buf.push_str(c);
                        mode = *prev_mode;
                    },
                },
            }
        }

        // Assert nothing is left unmatched
        match mode {
            ParseMode::Quotes(first_pos) => return Err(Error::UnmatchedQuote { first: first_pos, pos: args.len() }),
            ParseMode::Escaped(esc_pos, _) => return Err(Error::UnmatchedEscape { esc: esc_pos, pos: args.len() }),
            ParseMode::Start => {},
        }

        // Resolve the remaining buffer, if any
        if !buf.is_empty() {
            let (key, value): (String, Option<String>) = parse_arg(&self.keys, &buf, args.len() - buf.len())?;
            parsed_args.insert(key, value);
        }

        // Done, return the parsed arguments!
        Ok(parsed_args)
    }
}

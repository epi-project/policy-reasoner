//  VISUALIZE.rs
//    by Lut99
//
//  Created:
//    31 Oct 2023, 14:30:00
//  Last edited:
//    02 Nov 2023, 14:46:43
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a simple traversal over the [`Workflow`] to print it
//!   neatly to some writer.
//

use std::fmt::{Display, Formatter, Result as FResult};

use super::spec::{Elem, Workflow};


/***** HELPER MACROS *****/
/// Prints a given iterator somewhat nicely to a string.
macro_rules! write_iter {
    ($iter:expr, $conn:literal) => {{
        let mut iter = $iter.peekable();
        format!(
            "{}",
            if let Some(first) = iter.next() {
                if iter.peek().is_some() {
                    format!(concat!("{}", $conn, "{}"), first, iter.collect::<Vec<String>>().join($conn))
                } else {
                    format!("{}", first)
                }
            } else {
                String::from("<none>")
            }
        )
    }};
}





/***** HELPER FUNCTIONS *****/
/// Writes an [`Elem`] to the given formatter.
///
/// # Arguments
/// - `funcs`: A map of function IDs to metadata.
/// - `f`: The [`Formatter`] to write to.
/// - `elem`: The [`Elem`] to write.
/// - `prefix`: Some prefix to write before every line.
///
/// # Errors
/// This function only errors if we failed to write to the given `f`.
fn print_elem(f: &mut Formatter<'_>, elem: &'_ Elem, prefix: &dyn Display) -> FResult {
    // Print the element
    match elem {
        Elem::Task(t) => {
            writeln!(f, "{}task", prefix)?;
            writeln!(f, "{}  - name    : {}", prefix, t.name)?;
            writeln!(f, "{}  - package : {}", prefix, t.package)?;
            writeln!(f, "{}  - version : {}", prefix, t.version)?;
            writeln!(f, "{}  - hash    : {}", prefix, if let Some(hash) = &t.hash { hash.as_str() } else { "<unhashed>" })?;
            writeln!(f, "{}", prefix)?;
            writeln!(f, "{}  - input  : {}", prefix, write_iter!(t.input.iter().map(|data| format!("'{}'", data.name)), " or "))?;
            writeln!(
                f,
                "{}  - output : {}",
                prefix,
                if let Some(output) = &t.output { format!("'{}'", output.name.as_str()) } else { "<none>".into() }
            )?;
            writeln!(f, "{}", prefix)?;
            writeln!(f, "{}  - location  : {}", prefix, if let Some(location) = &t.location { location.as_str() } else { "<unplanned>" })?;
            writeln!(
                f,
                "{}  - metadata  : {}",
                prefix,
                write_iter!(
                    t.metadata.iter().map(|metadata| format!(
                        "{}.{} ({}, {})",
                        metadata.namespace,
                        metadata.tag,
                        metadata.signature,
                        if let Some(valid) = metadata.signature_valid { if valid { "OK" } else { "invalid" } } else { "<not validated>" }
                    )),
                    ", "
                )
            )?;
            writeln!(f, "{}  - signature : {}", prefix, t.signature)?;

            // Do next
            print_elem(f, &*t.next, prefix)
        },

        Elem::Branch(b) => {
            writeln!(f, "{}branch", prefix)?;

            // Write the branches
            for (i, branch) in b.branches.iter().enumerate() {
                writeln!(f, "{}{}<branch{}>", prefix, Indent(4), i)?;
                print_elem(f, branch, &Pair(prefix, Indent(8)))?;
                writeln!(f, "{}", prefix)?;
            }

            // Do next
            print_elem(f, &*b.next, prefix)
        },
        Elem::Parallel(p) => {
            writeln!(f, "{}parallel", prefix)?;

            // Write the branches
            for (i, branch) in p.branches.iter().enumerate() {
                writeln!(f, "{}{}<branch{}>", prefix, Indent(4), i)?;
                print_elem(f, branch, &Pair(prefix, Indent(8)))?;
                writeln!(f, "{}", prefix)?;
            }

            // Do next
            print_elem(f, &*p.next, prefix)
        },
        Elem::Loop(l) => {
            writeln!(f, "{}loop", prefix)?;
            writeln!(f, "{}<repeated>", Pair(prefix, Indent(4)))?;
            print_elem(f, &*l.body, &Pair(prefix, Indent(8)))?;
            writeln!(f)?;

            // Do next
            print_elem(f, &*l.next, prefix)
        },
        Elem::Commit(c) => {
            writeln!(f, "{}commit <{} as '{}'>", prefix, write_iter!(c.input.iter().map(|data| format!("'{}'", data.name)), " or "), c.data_name)?;

            // Do next
            print_elem(f, &*c.next, prefix)
        },

        Elem::Next => {
            writeln!(f, "{}next", prefix)
        },
        Elem::Stop(returns) => {
            writeln!(
                f,
                "{}stop{}",
                prefix,
                if !returns.is_empty() {
                    format!(" <returns {}>", write_iter!(returns.iter().map(|data| format!("'{}'", data.name)), " or "))
                } else {
                    String::new()
                }
            )
        },
    }
}





/***** HELPERS *****/
/// Writes two display things successively.
struct Pair<D1, D2>(D1, D2);
impl<D1: Display, D2: Display> Display for Pair<D1, D2> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult { write!(f, "{}{}", self.0, self.1) }
}

/// Generates indentation of the asked size.
struct Indent(usize);
impl Display for Indent {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        for _ in 0..self.0 {
            write!(f, " ")?;
        }
        Ok(())
    }
}





/***** FORMATTERS *****/
/// Capable of printing the [`Workflow`] to some writer.
#[derive(Debug)]
pub struct WorkflowFormatter<'w> {
    /// The workflow to format.
    wf: &'w Workflow,
}
impl<'w> Display for WorkflowFormatter<'w> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        // Print some nice header thingy
        writeln!(f, "Workflow [")?;

        // Alright print the main elements
        print_elem(f, &self.wf.start, &Indent(4))?;

        // Finish with the end bracket
        write!(f, "]")
    }
}





impl Workflow {
    /// Returns a nice formatter that visualizes the workflow more easily understandable than its [`Debug`](std::fmt::Debug)-implementation.
    ///
    /// # Returns
    /// A new [`WorkflowFormatter`] that can visualize the workflow when its [`Display`]-implementation is called.
    #[inline]
    pub fn visualize(&self) -> WorkflowFormatter { WorkflowFormatter { wf: self } }
}

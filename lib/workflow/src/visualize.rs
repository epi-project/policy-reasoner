//  VISUALIZE.rs
//    by Lut99
//
//  Created:
//    31 Oct 2023, 14:30:00
//  Last edited:
//    31 Oct 2023, 15:41:53
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a simple traversal over the [`Workflow`] to print it
//!   neatly to some writer.
//

use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FResult};

use super::spec::{Elem, Function, FunctionBody, Workflow};


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
fn print_elem(funcs: &HashMap<usize, &Function>, f: &mut Formatter<'_>, elem: &'_ Elem, prefix: &dyn Display) -> FResult {
    // Print the element
    match elem {
        Elem::Task(t) => {
            writeln!(f, "{}task", prefix)?;
            writeln!(f, "{}  - name    : {}", prefix, t.name)?;
            writeln!(f, "{}  - package : {}", prefix, t.package)?;
            writeln!(f, "{}  - version : {}", prefix, t.version)?;
            writeln!(f, "{}  - hash    : {}", prefix, if let Some(hash) = &t.hash { hash.as_str() } else { "<unhashed>" })?;
            writeln!(f, "{}", prefix)?;
            writeln!(f, "{}  - input  : {}", prefix, t.input.iter().map(|data| data.name.as_str()).collect::<Vec<&str>>().join(", "))?;
            writeln!(f, "{}  - output : {}", prefix, if let Some(output) = &t.output { output.name.as_str() } else { "<none>" })?;
            writeln!(f, "{}", prefix)?;
            writeln!(f, "{}  - location : {}", prefix, if let Some(location) = &t.location { location.as_str() } else { "<unplanned>" })?;
            writeln!(
                f,
                "{}  - metadata : {}",
                prefix,
                t.metadata
                    .iter()
                    .map(|metadata| format!(
                        "{}.{} ({}, {})",
                        metadata.namespace,
                        metadata.tag,
                        metadata.signature,
                        if let Some(valid) = metadata.signature_valid { if valid { "OK" } else { "invalid" } } else { "<not validated>" }
                    ))
                    .collect::<Vec<String>>()
                    .join(", ")
            )?;

            // Do next
            print_elem(funcs, f, &*t.next, prefix)
        },

        Elem::Branch(b) => {
            writeln!(f, "{}branch", prefix)?;

            // Write the branches
            for (i, branch) in b.branches.iter().enumerate() {
                writeln!(f, "{}{}<branch{}>", prefix, Indent(4), i)?;
                print_elem(funcs, f, branch, &Pair(prefix, Indent(8)))?;
                writeln!(f, "{}", prefix)?;
            }

            // Do next
            print_elem(funcs, f, &*b.next, prefix)
        },
        Elem::Parallel(p) => {
            writeln!(f, "{}parallel", prefix)?;

            // Write the branches
            for (i, branch) in p.branches.iter().enumerate() {
                writeln!(f, "{}{}<branch{}>", prefix, Indent(4), i)?;
                print_elem(funcs, f, branch, &Pair(prefix, Indent(8)))?;
                writeln!(f, "{}", prefix)?;
            }

            // Do next
            print_elem(funcs, f, &*p.next, prefix)
        },
        Elem::Loop(l) => {
            writeln!(f, "{}loop", prefix)?;
            writeln!(f, "{}<repeated>", Pair(prefix, Indent(4)))?;
            print_elem(funcs, f, &*l.body, &Pair(prefix, Indent(8)))?;
            writeln!(f)?;

            // Do next
            print_elem(funcs, f, &*l.next, prefix)
        },
        Elem::Call(c) => {
            writeln!(f, "{}call <{}:{}>", prefix, c.id, funcs.get(&c.id).map(|func| func.name.as_str()).unwrap_or("???"))?;

            // Do next
            print_elem(funcs, f, &*c.next, prefix)
        },

        Elem::Next => writeln!(f, "{}next", prefix),
        Elem::Return => writeln!(f, "{}return", prefix),
        Elem::Stop => writeln!(f, "{}stop", prefix),
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
        // Prepare a map of functions to metadata
        let funcs: HashMap<usize, &Function> = self.wf.funcs.iter().map(|(id, (func, _))| (*id, func)).collect();

        // Print some nice header thingy
        writeln!(f, "Workflow [")?;

        // Print the functions first
        let mut ids: Vec<usize> = self.wf.funcs.keys().cloned().collect();
        ids.sort();
        for id in ids {
            let (func, body): &(Function, FunctionBody) = self.wf.funcs.get(&id).unwrap();
            match body {
                FunctionBody::Elems(elem) => {
                    writeln!(f, "    {}:{}()", id, func.name)?;
                    print_elem(&funcs, f, &*elem.borrow(), &Indent(8))?
                },
                FunctionBody::Builtin => writeln!(f, "    {}:{}() <builtin>", id, func.name)?,
            }
            writeln!(f)?;
            writeln!(f)?;
        }

        // Alright print the main elements
        writeln!(f, "    <main>")?;
        print_elem(&funcs, f, &self.wf.start, &Indent(8))?;

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

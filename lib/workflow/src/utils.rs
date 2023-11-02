//  UTILS.rs
//    by Lut99
//
//  Created:
//    02 Nov 2023, 15:11:34
//  Last edited:
//    02 Nov 2023, 15:31:08
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines some utilities used in multiple places in this crate.
//

use std::fmt::{Display, Formatter, Result as FResult};
use std::panic::catch_unwind;

use brane_ast::ast;
use brane_ast::state::VirtualSymTable;


/***** FORMATTERS *****/
/// A static formatter for a [`ProgramCounter`] that shows it nicer.
#[derive(Clone, Debug)]
pub struct PrettyProgramCounter(pub usize, pub usize, pub Option<String>);
impl Display for PrettyProgramCounter {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        match &self.2 {
            Some(name) => write!(f, "{}:{}", name, self.1),
            None => write!(f, "{}:{}", self.0, self.1),
        }
    }
}





/***** LIBRARY FUNCTIONS *****/
/// Gets a workflow edge from a PC.
///
/// # Arguments
/// - `wir`: The [`ast::Workflow`] to get the edge from.
/// - `pc`: The program counter that points to the edge (hopefully).
///
/// # Returns
/// The edge the `pc` pointed to, or [`None`] if it was out-of-bounds.
#[inline]
pub fn get_edge(wir: &ast::Workflow, pc: impl AsRef<ProgramCounter>) -> Option<&ast::Edge> {
    let pc: &ProgramCounter = pc.as_ref();
    if pc.0 == usize::MAX { wir.graph.get(pc.1) } else { wir.funcs.get(&pc.0).map(|edges| edges.get(pc.1)).flatten() }
}





/***** LIBRARY *****/
/// Abstracts over a program counter.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProgramCounter(pub usize, pub usize);
impl Default for ProgramCounter {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl ProgramCounter {
    /// Creates a new ProgramCounter that points to the start of a workflow.
    ///
    /// # Returns
    /// Exactly what you expect, a ProgramCounter that, when used, points to the start of a workflow.
    #[inline]
    pub fn new() -> Self { Self(usize::MAX, 0) }

    /// Creates a new ProgramCounter that points to the start of a new function.
    ///
    /// # Argumetns
    /// - `func`: The function ID of the new function.
    ///
    /// # Returns
    /// A new ProgramCounter that points to the start of the given function.
    #[inline]
    pub fn call(func: usize) -> Self { Self(func, 0) }

    /// Creates a new ProgramCounter that points to the same function, next instruction.
    ///
    /// # Argumetns
    /// - `idx`: The new index in the same instruction.
    ///
    /// # Returns
    /// A new ProgramCounter that points to the given location in this function.
    #[inline]
    pub fn jump(&self, idx: usize) -> Self { Self(self.0, idx) }

    /// Returns a formatter that shows the function resolved to a name if possible.
    ///
    /// # Returns
    /// A [`PrettyProgramCounter`] that shows the name of the function indexed by `self.0` if known.
    #[inline]
    pub fn display(&self, table: &VirtualSymTable) -> PrettyProgramCounter {
        // Attempt to find the function ID
        if self.0 == usize::MAX {
            PrettyProgramCounter(self.0, self.1, Some("<main>".into()))
        } else if let Ok(def) = catch_unwind(|| table.func(self.0)) {
            PrettyProgramCounter(self.0, self.1, Some(def.name.clone()))
        } else {
            PrettyProgramCounter(self.0, self.1, None)
        }
    }
}
impl AsRef<ProgramCounter> for ProgramCounter {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl AsMut<ProgramCounter> for ProgramCounter {
    #[inline]
    fn as_mut(&mut self) -> &mut Self { self }
}
impl From<&ProgramCounter> for ProgramCounter {
    #[inline]
    fn from(value: &Self) -> Self { *value }
}
impl From<&mut ProgramCounter> for ProgramCounter {
    #[inline]
    fn from(value: &mut Self) -> Self { *value }
}
impl From<(usize, usize)> for ProgramCounter {
    #[inline]
    fn from(value: (usize, usize)) -> Self { Self(value.0, value.1) }
}
impl From<ProgramCounter> for (usize, usize) {
    #[inline]
    fn from(value: ProgramCounter) -> Self { (value.0, value.1) }
}

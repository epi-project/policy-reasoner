//  UTILS.rs
//    by Lut99
//
//  Created:
//    02 Nov 2023, 15:11:34
//  Last edited:
//    12 Jun 2024, 17:42:17
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines some utilities used in multiple places in this crate.
//

use brane_ast::ast;
use brane_exe::pc::ProgramCounter;


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
pub fn get_edge(wir: &ast::Workflow, pc: ProgramCounter) -> Option<&ast::Edge> {
    if pc.func_id.is_main() { wir.graph.get(pc.edge_idx) } else { wir.funcs.get(&pc.func_id.id()).and_then(|edges| edges.get(pc.edge_idx)) }
}

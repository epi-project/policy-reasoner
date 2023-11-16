//  COMPILE.rs
//    by Lut99
//
//  Created:
//    27 Oct 2023, 17:39:59
//  Last edited:
//    16 Nov 2023, 17:37:38
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines conversion functions between the
//!   [Checker Workflow](Workflow) and the [WIR](ast::Workflow).
//

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::panic::catch_unwind;

use brane_ast::spec::BuiltinFunctions;
use brane_ast::{ast, MergeStrategy};
use enum_debug::EnumDebug as _;
use log::{debug, trace, Level};
use rand::Rng as _;
use specifications::data::{AvailabilityKind, PreprocessKind};

use super::preprocess;
use super::spec::{Dataset, Elem, ElemBranch, ElemCommit, ElemLoop, ElemParallel, ElemTask, User, Workflow};
use super::utils::{self, PrettyProgramCounter, ProgramCounter};


/***** ERRORS *****/
/// Defines errors that may occur when compiling an [`ast::Workflow`] to a [`Workflow`].
#[derive(Debug)]
pub enum Error {
    /// Failed to preprocess the given workflow.
    Preprocess { err: super::preprocess::Error },
    /// Function ID was out-of-bounds.
    PcOutOfBounds { pc: PrettyProgramCounter, max: usize },
    /// A parallel edge was found who's `merge` was not found.
    ParallelMergeOutOfBounds { pc: PrettyProgramCounter, merge: PrettyProgramCounter },
    /// A parallel edge was found who's `merge` is not an [`ast::Edge::Join`].
    ParallelWithNonJoin { pc: PrettyProgramCounter, merge: PrettyProgramCounter, got: String },
    /// Found a join that wasn't paired with a parallel edge.
    StrayJoin { pc: PrettyProgramCounter },
    /// A call was performed to a non-builtin
    IllegalCall { pc: PrettyProgramCounter, name: String },
    /// A `commit_result()` was found that returns more than 1 result.
    CommitTooMuchOutput { pc: PrettyProgramCounter, got: usize },
    /// A `commit_result()` was found without output.
    CommitNoOutput { pc: PrettyProgramCounter },
    /// A `commit_result()` was found that outputs a result instead of a dataset.
    CommitReturnsResult { pc: PrettyProgramCounter },
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Error::*;
        match self {
            Preprocess { .. } => write!(f, "Failed to preprocess input WIR workflow"),
            PcOutOfBounds { pc, max } => write!(f, "Program counter {} is out-of-bounds (function {} has {} edges)", pc, pc.0, max),
            ParallelMergeOutOfBounds { pc, merge } => {
                write!(f, "Parallel edge at {pc}'s merge pointer {merge} is out-of-bounds")
            },
            ParallelWithNonJoin { pc, merge, got } => {
                write!(f, "Parallel edge at {pc}'s merge edge (at {merge}) was not an Edge::Join, but an Edge::{got}")
            },
            StrayJoin { pc } => write!(f, "Found Join-edge without preceding Parallel-edge at {pc}"),
            IllegalCall { pc, name } => {
                write!(f, "Encountered illegal call to function '{name}' at {pc} (calls to non-task, non-builtin functions are not supported)")
            },
            CommitTooMuchOutput { pc, got } => {
                write!(f, "Call to `commit_result()` as {pc} returns more than 1 outputs (got {got})")
            },
            CommitNoOutput { pc } => write!(f, "Call to `commit_result()` at {pc} does not return a dataset"),
            CommitReturnsResult { pc } => {
                write!(f, "Call to `commit_result()` at {pc} returns an IntermediateResult instead of a Data")
            },
        }
    }
}
impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use Error::*;
        match self {
            Preprocess { err, .. } => Some(err),
            PcOutOfBounds { .. }
            | ParallelMergeOutOfBounds { .. }
            | ParallelWithNonJoin { .. }
            | StrayJoin { .. }
            | IllegalCall { .. }
            | CommitTooMuchOutput { .. }
            | CommitNoOutput { .. }
            | CommitReturnsResult { .. } => None,
        }
    }
}





/***** HELPER FUNCTIONS *****/
/// Generates a random ID for a task/commit/workflow.
///
/// # Arguments
/// - `len`: The length of random characters to generate.
///
/// # Returns
/// A string with the new identifier.
#[inline]
fn generate_id(len: usize) -> String { rand::thread_rng().sample_iter(rand::distributions::Alphanumeric).take(len).map(char::from).collect() }

/// Reconstructs the workflow graph to [`Elem`]s instead of [`ast::Edge`]s.
///
/// # Arguments
/// - `wir`: The [`ast::Workflow`] to analyse.
/// - `wf_id`: The identifier of the workflow we're compiling in.
/// - `table`: A running [`ast::SymTable`] that determines the current types in scope.
/// - `calls`: The map of Call program-counter-indices to function IDs called.
/// - `pc`: The program-counter-index of the edge to analyse. These are pairs of `(function, edge_idx)`, where main is referred to by [`usize::MAX`](usize).
/// - `plug`: The element to write when we reached the (implicit) end of a branch.
/// - `breakpoint`: An optional program-counter-index that, if given, will not analyse that edge onwards (excluding it too).
///
/// # Returns
/// An [`Elem`] representing the given branch of the workflow.
///
/// # Errors
/// This function errors if a definition in the Workflow was unknown.
fn reconstruct_graph(
    wir: &ast::Workflow,
    wf_id: &str,
    table: &ast::SymTable,
    calls: &HashMap<ProgramCounter, usize>,
    pc: ProgramCounter,
    plug: Elem,
    breakpoint: Option<ProgramCounter>,
) -> Result<Elem, Error> {
    // Stop if we hit the breakpoint
    if let Some(breakpoint) = breakpoint {
        if pc == breakpoint {
            return Ok(plug);
        }
    }

    // Get the edge we're talking about
    let edge: &ast::Edge = match utils::get_edge(wir, pc) {
        Some(edge) => edge,
        None => return Ok(plug),
    };

    // Match the edge
    trace!("Compiling {:?}", edge.variant());
    match edge {
        ast::Edge::Linear { next, .. } => {
            // Simply skip to the next, as linear connectors are no longer interesting
            reconstruct_graph(wir, wf_id, table, calls, pc.jump(*next), plug, breakpoint)
        },

        ast::Edge::Node { task, locs: _, at, input, result, next } => {
            // Resolve the task definition
            let def: &ast::ComputeTaskDef = match catch_unwind(|| table.task(*task)) {
                Ok(def) => {
                    if let ast::TaskDef::Compute(c) = def {
                        c
                    } else {
                        unimplemented!();
                    }
                },
                Err(_) => panic!("Encountered unknown task '{task}' after preprocessing"),
            };

            // Return the elem
            Ok(Elem::Task(ElemTask {
                id: format!("{}-{}-task", wf_id, pc.display(table)),
                name: def.function.name.clone(),
                package: def.package.clone(),
                version: def.version,
                input: input
                    .iter()
                    .map(|(name, avail)| Dataset {
                        name: name.name().into(),
                        from: avail
                            .as_ref()
                            .map(|avail| match avail {
                                AvailabilityKind::Available { how: _ } => None,
                                AvailabilityKind::Unavailable { how: PreprocessKind::TransferRegistryTar { location, address: _ } } => {
                                    Some(location.clone())
                                },
                            })
                            .flatten(),
                    })
                    .collect(),
                output: result.as_ref().map(|name| Dataset { name: name.clone(), from: None }),
                location: at.clone(),
                metadata: vec![],
                next: Box::new(reconstruct_graph(wir, wf_id, table, calls, pc.jump(*next), plug, breakpoint)?),
            }))
        },

        ast::Edge::Stop {} => Ok(Elem::Stop(HashSet::new())),

        ast::Edge::Branch { true_next, false_next, merge } => {
            // Construct the branches first
            let mut branches: Vec<Elem> =
                vec![reconstruct_graph(wir, wf_id, table, calls, pc.jump(*true_next), Elem::Next, merge.map(|merge| pc.jump(merge)))?];
            if let Some(false_next) = false_next {
                branches.push(reconstruct_graph(wir, wf_id, table, calls, pc.jump(*false_next), Elem::Next, merge.map(|merge| pc.jump(merge)))?)
            }

            // Build the next, if there is any
            let next: Elem = merge
                .map(|merge| reconstruct_graph(wir, wf_id, table, calls, pc.jump(merge), plug, breakpoint))
                .transpose()?
                .unwrap_or(Elem::Stop(HashSet::new()));

            // Build the elem using those branches and next
            Ok(Elem::Branch(ElemBranch { branches, next: Box::new(next) }))
        },

        ast::Edge::Parallel { branches, merge } => {
            // Construct the branches first
            let mut elem_branches: Vec<Elem> = Vec::with_capacity(branches.len());
            for branch in branches {
                elem_branches.push(reconstruct_graph(wir, wf_id, table, calls, pc.jump(*branch), Elem::Next, Some(pc.jump(*merge)))?);
            }

            // Let us checkout that the merge point is a join
            let merge_edge: &ast::Edge = match utils::get_edge(wir, pc.jump(*merge)) {
                Some(edge) => edge,
                None => return Err(Error::ParallelMergeOutOfBounds { pc: pc.display(table), merge: pc.jump(*merge).display(table) }),
            };
            let (strategy, next): (MergeStrategy, usize) = if let ast::Edge::Join { merge, next } = merge_edge {
                (*merge, *next)
            } else {
                return Err(Error::ParallelWithNonJoin {
                    pc:    pc.display(table),
                    merge: pc.jump(*merge).display(table),
                    got:   merge_edge.variant().to_string(),
                });
            };

            // Build the post-join point onwards
            let next: Elem = reconstruct_graph(wir, wf_id, table, calls, pc.jump(next), plug, breakpoint)?;

            // We have enough to build ourselves
            Ok(Elem::Parallel(ElemParallel { branches: elem_branches, merge: strategy, next: Box::new(next) }))
        },

        ast::Edge::Join { .. } => Err(Error::StrayJoin { pc: pc.display(table) }),

        ast::Edge::Loop { cond, body, next } => {
            // Build the body first
            let body_elems: Elem = reconstruct_graph(wir, wf_id, table, calls, pc.jump(*body), Elem::Next, Some(pc.jump(*cond)))?;

            // Build the condition, with immediately following the body for any open ends that we find
            let cond: Elem = reconstruct_graph(wir, wf_id, table, calls, pc.jump(*cond), body_elems, Some(pc.jump(*body - 1)))?;

            // Build the next
            let next: Elem = next
                .map(|next| reconstruct_graph(wir, wf_id, table, calls, pc.jump(next), plug, breakpoint))
                .transpose()?
                .unwrap_or(Elem::Stop(HashSet::new()));

            // We have enough to build self
            Ok(Elem::Loop(ElemLoop { body: Box::new(cond), next: Box::new(next) }))
        },

        ast::Edge::Call { input, result, next } => {
            // Attempt to get the call ID & matching definition
            let func_def: &ast::FunctionDef = match calls.get(&pc) {
                Some(id) => match catch_unwind(|| table.func(*id)) {
                    Ok(def) => def,
                    Err(_) => panic!("Encountered unknown function '{id}' after preprocessing"),
                },
                None => panic!("Encountered unresolved call after preprocessing"),
            };

            // Only allow calls to builtins
            if func_def.name == BuiltinFunctions::CommitResult.name() {
                // Attempt to fetch the name of the dataset
                if result.len() > 1 {
                    return Err(Error::CommitTooMuchOutput { pc: pc.display(table), got: result.len() });
                }
                let data_name: String = if let Some(name) = result.iter().next() {
                    if let ast::DataName::Data(name) = name {
                        name.clone()
                    } else {
                        return Err(Error::CommitReturnsResult { pc: pc.display(table) });
                    }
                } else {
                    return Err(Error::CommitNoOutput { pc: pc.display(table) });
                };

                // Construct next first
                let next: Elem = reconstruct_graph(wir, wf_id, table, calls, pc.jump(*next), plug, breakpoint)?;

                // Then we wrap the rest in a commit
                Ok(Elem::Commit(ElemCommit {
                    id: format!("{}-{}-commit", wf_id, pc.display(table)),
                    data_name,
                    input: input.iter().map(|input| Dataset { name: input.name().into(), from: None }).collect(),
                    next: Box::new(next),
                }))
            } else if func_def.name == BuiltinFunctions::Print.name()
                || func_def.name == BuiltinFunctions::PrintLn.name()
                || func_def.name == BuiltinFunctions::Len.name()
            {
                // Using them is OK, we just ignore them for the improved workflow
                reconstruct_graph(wir, wf_id, table, calls, pc.jump(*next), plug, breakpoint)
            } else {
                Err(Error::IllegalCall { pc: pc.display(table), name: func_def.name.clone() })
            }
        },

        ast::Edge::Return { result } => Ok(Elem::Stop(result.iter().map(|data| Dataset { name: data.name().into(), from: None }).collect())),
    }
}





/***** LIBRARY *****/
impl TryFrom<ast::Workflow> for Workflow {
    type Error = Error;

    #[inline]
    fn try_from(value: ast::Workflow) -> Result<Self, Self::Error> {
        // First, analyse the calls in the workflow as much as possible (and simplify)
        let (wir, calls): (ast::Workflow, HashMap<ProgramCounter, usize>) = match preprocess::simplify(value) {
            Ok(res) => res,
            Err(err) => return Err(Error::Preprocess { err }),
        };
        if log::max_level() >= Level::Debug {
            // Write the processed graph
            let mut buf: Vec<u8> = vec![];
            brane_ast::traversals::print::ast::do_traversal(&wir, &mut buf).unwrap();
            debug!("Preprocessed workflow:\n\n{}\n", String::from_utf8_lossy(&buf));
        }

        // Alright now attempt to re-build the graph in the new style
        let wf_id: String = format!("workflow-{}", generate_id(8));
        let graph: Elem = reconstruct_graph(&wir, &wf_id, &wir.table, &calls, ProgramCounter::new(), Elem::Stop(HashSet::new()), None)?;

        // Build a new Workflow with that!
        Ok(Self {
            id:    wf_id,
            start: graph,

            user:      User { name: "Danny Data Scientist".into() },
            metadata:  vec![],
            signature: "its_signed_i_swear_mom".into(),
        })
    }
}

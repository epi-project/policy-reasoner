//  COMPILE.rs
//    by Lut99
//
//  Created:
//    27 Oct 2023, 17:39:59
//  Last edited:
//    31 Oct 2023, 17:25:15
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines conversion functions between the
//!   [Checker Workflow](Workflow) and the [WIR](ast::Workflow).
//

use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::panic::catch_unwind;
use std::rc::Rc;

use brane_ast::state::VirtualSymTable;
use brane_ast::{ast, MergeStrategy};
use enum_debug::EnumDebug as _;
use log::trace;
use specifications::data::{AvailabilityKind, PreprocessKind};

use super::spec::{Dataset, Elem, ElemBranch, ElemCall, ElemLoop, ElemParallel, ElemTask, Function, FunctionBody, User, Workflow};


/***** ERRORS *****/
/// Defines errors that may occur when compiling an [`ast::Workflow`] to a [`Workflow`].
#[derive(Debug)]
pub enum Error {
    /// Unknown task given.
    UnknownTask { id: usize },
    /// Unknown function given.
    UnknownFunc { id: usize },
    /// Unknown variable given.
    UnknownVar { id: usize },
    /// A [`Call`](ast::Edge::Call)-edge was encountered while we didn't know of a function ID on the stack.
    CallingWithoutId { pc: (usize, usize) },

    /// Function ID was out-of-bounds.
    PcOutOfBounds { pc: (usize, usize), max: usize },
    /// A parallel edge was found who's `merge` was not found.
    ParallelMergeOutOfBounds { pc: (usize, usize), merge: (usize, usize) },
    /// A parallel edge was found who's `merge` is not an [`ast::Edge::Join`].
    ParallelWithNonJoin { pc: (usize, usize), merge: (usize, usize), got: String },
    /// Found a join that wasn't paired with a parallel edge.
    StrayJoin { pc: (usize, usize) },
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Error::*;
        match self {
            UnknownTask { id } => write!(f, "Encountered unknown task ID {id} in Node"),
            UnknownFunc { id } => write!(f, "Encountered unknown function ID {id} in Call"),
            UnknownVar { id } => write!(f, "Encountered unknown variable ID {id} in Linear (instruction)"),
            CallingWithoutId { pc } => write!(f, "Attempted to call function at ({},{}) without statically known task ID on the stack", pc.0, pc.1),

            PcOutOfBounds { pc, max } => write!(f, "Program counter ({},{}) is out-of-bounds for function {} with {} edges", pc.0, pc.1, pc.0, max),
            ParallelMergeOutOfBounds { pc, merge } => {
                write!(f, "Parallel edge at ({},{})'s merge pointer ({},{}) is out-of-bounds", pc.0, pc.1, merge.0, merge.1)
            },
            ParallelWithNonJoin { pc, merge, got } => write!(
                f,
                "Parallel edge at ({},{})'s merge edge (at ({},{})) was not an Edge::Join, but an Edge::{}",
                pc.0, pc.1, merge.0, merge.1, got
            ),
            StrayJoin { pc } => write!(f, "Found Join-edge without preceding Parallel-edge at ({},{})", pc.0, pc.1),
        }
    }
}
impl error::Error for Error {}





/***** HELPER FUNCTIONS *****/
/// Gets a workflow edge from a PC.
///
/// # Arguments
/// - `wir`: The [`ast::Workflow`] to get the edge from.
/// - `pc`: The program counter that points to the edge (hopefully).
///
/// # Returns
/// The edge the `pc` pointed to, or [`None`] if it was out-of-bounds.
#[inline]
fn get_edge(wir: &ast::Workflow, pc: (usize, usize)) -> Option<&ast::Edge> {
    if pc.0 == usize::MAX { wir.graph.get(pc.1) } else { wir.funcs.get(&pc.0).map(|edges| edges.get(pc.1)).flatten() }
}

/// Checks whether the given stream of instructions would end with a function ID on top of the stack.
///
/// # Arguments
/// - `instrs`: The list of instructions to analyse.
/// - `idx`: The index of the particular instruction (i.e., the previous one) to examine. When calling this functio non-recursively, use the **last** instruction.
///
/// # Returns
/// A double [`Option`] detailling what's possible:
/// - [`Some(Some(...))`] means that there was a function ID on top.
/// - [`Some(None)`] means that we _know_ there is _no_ function ID on top.
/// - [`None`] means that nothing was pushed, i.e., whatever was on top is still on top.
fn pushes_func_id(instrs: &[ast::EdgeInstr], idx: usize) -> Option<Option<usize>> {
    // Pop the next instruction
    let instr: &ast::EdgeInstr = if idx < instrs.len() {
        &instrs[idx]
    } else {
        // If we reached the last instruction, then we know no value was pushed :celebrate:
        return None;
    };

    // Examine what it does
    // NOTE: The BraneScript compiler only supports function calls over identifiers and projections. So we can ignore gnarly array stuff etc!
    // NOTE: Actually... we know violently little statically of class calls in general, because they are fully pushed to dynamic land. We _could_ learn it by tracking
    //       a variable's contents over multiple edges, but that fucks; let's give up and only support direct calls for now.
    match instr {
        // What we're looking for!
        ast::EdgeInstr::Function { def } => Some(Some(*def)),

        // Things instructions only pop, potentially (accidentally) removing our function
        // Jep just tell the thign we don't know, we don't need it for direct function calls
        ast::EdgeInstr::Pop {} | ast::EdgeInstr::PopMarker {} | ast::EdgeInstr::DynamicPop {} | ast::EdgeInstr::VarSet { .. } => Some(None),

        // Alright some weird local branching; fuck it, also give up because we don't know which of the branches will do it
        ast::EdgeInstr::Branch { .. } | ast::EdgeInstr::BranchNot { .. } => Some(None),

        // These instructions never pop- or push anything
        ast::EdgeInstr::VarDec { .. } | ast::EdgeInstr::VarUndec { .. } => Some(None),

        // These instructions push invalid things _for sure_
        ast::EdgeInstr::Cast { .. }
        | ast::EdgeInstr::Not {}
        | ast::EdgeInstr::Neg {}
        | ast::EdgeInstr::And {}
        | ast::EdgeInstr::Or {}
        | ast::EdgeInstr::Add {}
        | ast::EdgeInstr::Sub {}
        | ast::EdgeInstr::Mul {}
        | ast::EdgeInstr::Div {}
        | ast::EdgeInstr::Mod {}
        | ast::EdgeInstr::Eq {}
        | ast::EdgeInstr::Ne {}
        | ast::EdgeInstr::Lt {}
        | ast::EdgeInstr::Le {}
        | ast::EdgeInstr::Gt {}
        | ast::EdgeInstr::Ge {}
        | ast::EdgeInstr::Array { .. }
        | ast::EdgeInstr::ArrayIndex { .. }
        | ast::EdgeInstr::Instance { .. }
        | ast::EdgeInstr::Proj { .. }
        | ast::EdgeInstr::VarGet { .. }
        | ast::EdgeInstr::Boolean { .. }
        | ast::EdgeInstr::Integer { .. }
        | ast::EdgeInstr::Real { .. }
        | ast::EdgeInstr::String { .. } => Some(None),
    }
}

/// Analyses the edges in an [`ast::Workflow`] to resolve function calls to the ID of the functions they call.
///
/// # Arguments
/// - `wir`: The [`ast::Workflow`] to analyse.
/// - `table`: A running [`VirtualSymTable`] that determines the current types in scope.
/// - `stack_id`: The function ID currently known to be on the stack. Is [`None`] if we don't know this.
/// - `pc`: The program-counter-index of the edge to analyse. These are pairs of `(function, edge_idx)`, where main is referred to by [`usize::MAX`](usize).
/// - `breakpoint`: An optional program-counter-index that, if given, will not analyse that edge onwards (excluding it too).
///
/// # Returns
/// A tuple with a [`HashMap`] that maps call indices (as program-counter-indices) to function IDs and an optional top call ID currently on the stack.
///
/// Note that, if a call ID occurs in the map but has [`None`] as function ID, it means it does not map to a body (e.g., a builtin).
///
/// # Errors
/// This function may error if we failed to statically discover the function IDs.
fn resolve_calls(
    wir: &ast::Workflow,
    table: &mut VirtualSymTable,
    pc: (usize, usize),
    stack_id: Option<usize>,
    breakpoint: Option<(usize, usize)>,
) -> Result<(HashMap<(usize, usize), usize>, Option<usize>), Error> {
    // Quit if we're at the breakpoint
    if let Some(breakpoint) = breakpoint {
        if pc == breakpoint {
            return Ok((HashMap::new(), None));
        }
    }

    // Get the edge in the workflow
    let edge: &ast::Edge = match get_edge(wir, pc) {
        Some(edge) => edge,
        None => return Ok((HashMap::new(), None)),
    };

    // Match to recursively process it
    trace!("Analyzing {:?} calls", edge.variant());
    match edge {
        ast::Edge::Node { task, next, .. } => {
            // Attempt to discover the return type of the Node.
            let def: &ast::TaskDef = match std::panic::catch_unwind(|| table.task(*task)) {
                Ok(def) => def,
                Err(_) => return Err(Error::UnknownTask { id: *task }),
            };

            // Alright, recurse with the next instruction
            resolve_calls(wir, table, (pc.0, *next), if def.func().ret.is_void() { stack_id } else { None }, breakpoint)
        },

        ast::Edge::Linear { instrs, next } => {
            // Analyse the instructions to find out if we can deduce a new `stack_id`
            let stack_id: Option<usize> = if !instrs.is_empty() { pushes_func_id(instrs, instrs.len() - 1).unwrap_or(stack_id) } else { stack_id };

            // Analyse the next one
            resolve_calls(wir, table, (pc.0, *next), stack_id, breakpoint)
        },

        ast::Edge::Stop {} => Ok((HashMap::new(), None)),

        ast::Edge::Branch { true_next, false_next, merge } => {
            // First, analyse the branches
            let (mut calls, mut stack_id): (HashMap<_, _>, Option<usize>) =
                resolve_calls(wir, table, (pc.0, *true_next), stack_id, merge.map(|merge| (pc.0, merge)))?;
            if let Some(false_next) = false_next {
                let (false_calls, false_stack) = resolve_calls(wir, table, (pc.0, *false_next), stack_id, merge.map(|merge| (pc.0, merge)))?;
                calls.extend(false_calls);
                if stack_id != false_stack {
                    stack_id = None;
                }
            }

            // Analyse the remaining part next
            if let Some(merge) = merge {
                let (merge_calls, merge_stack) = resolve_calls(wir, table, (pc.0, *merge), stack_id, breakpoint)?;
                calls.extend(merge_calls);
                stack_id = merge_stack;
            }

            // Alright, return the found results
            Ok((calls, stack_id))
        },

        ast::Edge::Parallel { branches, merge } => {
            // Simply analyse all branches first. No need to worry about their return values and such, since that's not until the `Join`.
            let mut calls: HashMap<_, _> = HashMap::new();
            for branch in branches {
                calls.extend(resolve_calls(wir, table, (pc.0, *branch), stack_id, breakpoint)?.0);
            }

            // OK, then analyse the rest assuming the stack is unchanged (we can do that because the parallel's branches get clones)
            let (new_calls, stack_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, (pc.0, *merge), stack_id, breakpoint)?;
            calls.extend(new_calls);
            Ok((calls, stack_id))
        },

        ast::Edge::Join { merge, next } => {
            // Simply do the next, only _not_ resetting the stack ID if no value is returned.
            resolve_calls(wir, table, (pc.0, *next), if *merge == MergeStrategy::None { stack_id } else { None }, breakpoint)
        },

        ast::Edge::Loop { cond, body, next } => {
            // Traverse the three individually, using the stack ID of the codebody that precedes it
            let (mut calls, mut cond_id): (HashMap<_, _>, Option<usize>) =
                resolve_calls(wir, table, (pc.0, *cond), stack_id, Some((pc.0, *body - 1)))?;
            let (body_calls, _): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, (pc.0, *body), cond_id, Some((pc.0, *cond)))?;
            calls.extend(body_calls);
            if let Some(next) = next {
                let (next_calls, next_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, (pc.0, *next), cond_id, breakpoint)?;
                calls.extend(next_calls);
                cond_id = next_id;
            }

            // Done!
            Ok((calls, cond_id))
        },

        ast::Edge::Call { next } => {
            // Alright time to jump functions based on the current top-of-the-stack
            let stack_id: usize = match stack_id {
                Some(id) => id,
                None => {
                    return Err(Error::CallingWithoutId { pc });
                },
            };

            // Get the function definition to extend the VirtualSymTable
            let def: &ast::FunctionDef = match catch_unwind(|| table.func(stack_id)) {
                Ok(def) => def,
                Err(_) => return Err(Error::UnknownFunc { id: stack_id }),
            };

            // Add the mapping to the table
            let mut calls: HashMap<(usize, usize), usize> = HashMap::from([(pc, stack_id)]);

            // Resolve the call of the function (builtins simply return nothing, so are implicitly handled)
            table.push(&def.table);
            let (call_calls, call_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, (stack_id, 0), None, None)?;
            table.pop();
            calls.extend(call_calls);

            // Then continue with the next one
            let (next_calls, next_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, (pc.0, *next), call_id, breakpoint)?;
            calls.extend(next_calls);
            Ok((calls, next_id))
        },

        ast::Edge::Return {} => {
            // If we're in the main function, this acts as an [`Elem::Stop`] with value
            if pc.0 == usize::MAX {
                return Ok((HashMap::new(), None));
            }

            // To see whether we pass a function ID, consult the function definition
            let def: &ast::FunctionDef = match catch_unwind(|| table.func(pc.0)) {
                Ok(def) => def,
                Err(_) => return Err(Error::UnknownFunc { id: pc.0 }),
            };

            // Only return the current one if the function returns void
            if def.ret.is_void() { Ok((HashMap::new(), stack_id)) } else { Ok((HashMap::new(), None)) }
        },
    }
}

/// Reconstructs the workflow graph to [`Elem`]s instead of [`ast::Edge`]s.
///
/// # Arguments
/// - `wir`: The [`ast::Workflow`] to analyse.
/// - `table`: A running [`VirtualSymTable`] that determines the current types in scope.
/// - `calls`: The map of Call program-counter-indices to function IDs called.
/// - `funcs`: A map of function bodies we've build for functions.
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
    table: &mut VirtualSymTable,
    calls: &HashMap<(usize, usize), usize>,
    funcs: &mut HashMap<usize, FunctionBody>,
    pc: (usize, usize),
    plug: Elem,
    breakpoint: Option<(usize, usize)>,
) -> Result<Elem, Error> {
    // Stop if we hit the breakpoint
    if let Some(breakpoint) = breakpoint {
        if pc == breakpoint {
            return Ok(plug);
        }
    }

    // Get the edge we're talking about
    let edge: &ast::Edge = match get_edge(wir, pc) {
        Some(edge) => edge,
        None => return Ok(plug),
    };

    // Match the edge
    trace!("Compiling {:?}", edge.variant());
    match edge {
        ast::Edge::Linear { next, .. } => {
            // Simply skip to the next, as linear connectors are no longer interesting
            reconstruct_graph(wir, table, calls, funcs, (pc.0, *next), plug, breakpoint)
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
                Err(_) => return Err(Error::UnknownTask { id: *task }),
            };

            // Return the elem
            Ok(Elem::Task(ElemTask {
                name:      def.function.name.clone(),
                package:   def.package.clone(),
                version:   def.version,
                hash:      None,
                input:     input
                    .iter()
                    .map(|(name, avail)| Dataset {
                        name:     name.name().into(),
                        from:     avail
                            .as_ref()
                            .map(|avail| match avail {
                                AvailabilityKind::Available { how: _ } => None,
                                AvailabilityKind::Unavailable { how: PreprocessKind::TransferRegistryTar { location, address: _ } } => {
                                    Some(location.clone())
                                },
                            })
                            .flatten(),
                        metadata: vec![],
                    })
                    .collect(),
                output:    result.as_ref().map(|name| Dataset { name: name.clone(), from: None, metadata: vec![] }),
                location:  at.clone(),
                metadata:  vec![],
                signature: "its_signed_i_swear_mom".into(),
                next:      Box::new(reconstruct_graph(wir, table, calls, funcs, (pc.0, *next), plug, breakpoint)?),
            }))
        },

        ast::Edge::Stop {} => Ok(Elem::Stop),

        ast::Edge::Branch { true_next, false_next, merge } => {
            // Construct the branches first
            let mut branches: Vec<Elem> =
                vec![reconstruct_graph(wir, table, calls, funcs, (pc.0, *true_next), Elem::Next, merge.map(|merge| (pc.0, merge)))?];
            if let Some(false_next) = false_next {
                branches.push(reconstruct_graph(wir, table, calls, funcs, (pc.0, *false_next), Elem::Next, merge.map(|merge| (pc.0, merge)))?)
            }

            // Build the next, if there is any
            let next: Elem =
                merge.map(|merge| reconstruct_graph(wir, table, calls, funcs, (pc.0, merge), plug, breakpoint)).transpose()?.unwrap_or(Elem::Stop);

            // Build the elem using those branches and next
            Ok(Elem::Branch(ElemBranch { branches, next: Box::new(next) }))
        },

        ast::Edge::Parallel { branches, merge } => {
            // Construct the branches first
            let mut elem_branches: Vec<Elem> = Vec::with_capacity(branches.len());
            for branch in branches {
                elem_branches.push(reconstruct_graph(wir, table, calls, funcs, (pc.0, *branch), Elem::Next, Some((pc.0, *merge)))?);
            }

            // Let us checkout that the merge point is a join
            let merge_edge: &ast::Edge = match get_edge(wir, (pc.0, *merge)) {
                Some(edge) => edge,
                None => return Err(Error::ParallelMergeOutOfBounds { pc, merge: (pc.0, *merge) }),
            };
            let (strategy, next): (MergeStrategy, usize) = if let ast::Edge::Join { merge, next } = merge_edge {
                (*merge, *next)
            } else {
                return Err(Error::ParallelWithNonJoin { pc, merge: (pc.0, *merge), got: merge_edge.variant().to_string() });
            };

            // Build the post-join point onwards
            let next: Elem = reconstruct_graph(wir, table, calls, funcs, (pc.0, next), plug, breakpoint)?;

            // We have enough to build ourselves
            Ok(Elem::Parallel(ElemParallel { branches: elem_branches, merge: strategy, next: Box::new(next) }))
        },

        ast::Edge::Join { .. } => Err(Error::StrayJoin { pc }),

        ast::Edge::Loop { cond, body, next } => {
            // Build the body first
            let body_elems: Elem = reconstruct_graph(wir, table, calls, funcs, (pc.0, *body), Elem::Next, Some((pc.0, *cond)))?;

            // Build the condition, with immediately following the body for any open ends that we find
            let cond: Elem = reconstruct_graph(wir, table, calls, funcs, (pc.0, *cond), body_elems, Some((pc.0, *body - 1)))?;

            // Build the next
            let next: Elem =
                next.map(|next| reconstruct_graph(wir, table, calls, funcs, (pc.0, next), plug, breakpoint)).transpose()?.unwrap_or(Elem::Stop);

            // We have enough to build self
            Ok(Elem::Loop(ElemLoop { body: Box::new(cond), next: Box::new(next) }))
        },

        ast::Edge::Call { next } => {
            // Attempt to get the call ID & matching definition
            let (func_id, func_def): (usize, &ast::FunctionDef) = match calls.get(&pc) {
                Some(id) => match catch_unwind(|| table.func(*id)) {
                    Ok(def) => (*id, def),
                    Err(_) => return Err(Error::UnknownFunc { id: *id }),
                },
                None => return Err(Error::CallingWithoutId { pc }),
            };

            // If there is a function body (i.e., it's not a builtin), then process that
            let func: FunctionBody = match funcs.get(&func_id) {
                // We already compiled this body, just fetch that
                Some(body) => body.clone(),
                // Otherwise, build it if it has a body
                None => {
                    if wir.funcs.contains_key(&func_id) {
                        // Wrap the call to the body in the extended symbol table for that function
                        table.push(&func_def.table);
                        let elem: FunctionBody = reconstruct_graph(wir, table, calls, funcs, (func_id, 0), Elem::Next, None)
                            .map(|elem| FunctionBody::Elems(Rc::new(RefCell::new(elem))))?;
                        table.pop();
                        funcs.insert(func_id, elem.clone());
                        elem
                    } else {
                        funcs.insert(func_id, FunctionBody::Builtin);
                        FunctionBody::Builtin
                    }
                },
            };

            // Process the next
            let next: Elem = reconstruct_graph(wir, table, calls, funcs, (pc.0, *next), plug, breakpoint)?;

            // Build self!
            Ok(Elem::Call(ElemCall { id: func_id, func, next: Box::new(next) }))
        },

        ast::Edge::Return {} => Ok(Elem::Return),
    }
}





/***** LIBRARY *****/
impl TryFrom<ast::Workflow> for Workflow {
    type Error = Error;

    #[inline]
    fn try_from(value: ast::Workflow) -> Result<Self, Self::Error> {
        // First, analyse the calls in the workflow as much as possible
        let (calls, _): (HashMap<(usize, usize), usize>, _) =
            resolve_calls(&value, &mut VirtualSymTable::with(&value.table), (usize::MAX, 0), None, None)?;

        // Alright now attempt to re-build the graph in the new style
        let mut funcs: HashMap<usize, FunctionBody> = HashMap::new();
        let graph: Elem = reconstruct_graph(&value, &mut VirtualSymTable::with(&value.table), &calls, &mut funcs, (usize::MAX, 0), Elem::Stop, None)?;

        // Build a new Workflow with that!
        Ok(Self {
            start: graph,
            funcs: funcs.into_iter().map(|(id, body)| (id, (Function { name: value.table.funcs[id].name.clone() }, body))).collect(),

            user:      User { name: "Danny Data Scientist".into(), metadata: vec![] },
            metadata:  vec![],
            signature: "its_signed_i_swear_mom".into(),
        })
    }
}

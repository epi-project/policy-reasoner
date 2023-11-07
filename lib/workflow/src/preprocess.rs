//  PREPROCESS.rs
//    by Lut99
//
//  Created:
//    02 Nov 2023, 14:52:26
//  Last edited:
//    07 Nov 2023, 11:10:44
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines a preprocessing step on a [WIR](Workflow) that simplifies it
//!   to increase the support of the simpler checker workflow.
//

use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::panic::catch_unwind;

use brane_ast::ast::{Edge, EdgeInstr, FunctionDef, TaskDef, Workflow};
use brane_ast::spec::BuiltinFunctions;
use brane_ast::state::VirtualSymTable;
use brane_ast::MergeStrategy;
use enum_debug::EnumDebug as _;
use log::trace;

use super::utils::{self, PrettyProgramCounter, ProgramCounter};


/***** TESTS *****/
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use brane_ast::traversals::print::ast;
    use brane_ast::{compile_program, CompileResult, ParserOptions};
    use brane_shr::utilities::{create_data_index_from, create_package_index_from};
    use specifications::data::DataIndex;
    use specifications::package::PackageIndex;

    use super::*;


    /// Runs checks to verify the workflow inlining analysis
    #[test]
    fn test_checker_workflow_inline() {
        // Defines a few test files with expected inlinable functions
        let tests: [(&str, HashMap<usize, bool>); 4] = [
            (r#"println("Hello, world!");"#, HashMap::from([(1, false)])),
            (r#"func hello_world() { return "Hello, world!"; } println(hello_world());"#, HashMap::from([(1, false), (4, true)])),
            (r#"import hello_world; println(hello_world());"#, HashMap::from([(1, false)])),
            (
                r#"func hello_world(n) { if (n <= 0) { return "Hello, world!"; } else { return "Hello, " + hello_world(n - 1) + "\n"; } } println(hello_world(3));"#,
                HashMap::from([(1, false), (4, false)]),
            ),
        ];

        // Load example package- and data indices
        let tests_path: PathBuf = PathBuf::from(super::super::TESTS_DIR);
        let pindex: PackageIndex = create_package_index_from(tests_path.join("packages"));
        let dindex: DataIndex = create_data_index_from(tests_path.join("data"));

        // Test them each
        for (i, (test, gold)) in tests.into_iter().enumerate() {
            // Compile to BraneScript (we'll assume this works)
            let wir: Workflow = match compile_program(test.as_bytes(), &pindex, &dindex, &ParserOptions::bscript()) {
                CompileResult::Workflow(wir, _) => wir,
                CompileResult::Err(errs) => {
                    for err in errs {
                        err.prettyprint(format!("<test {i}>"), test);
                    }
                    panic!("Failed to compile BraneScript (see error above)");
                },
                CompileResult::Eof(err) => {
                    err.prettyprint(format!("<test {i}>"), test);
                    panic!("Failed to compile BraneScript (see error above)");
                },

                _ => {
                    unreachable!();
                },
            };
            // Emit the compiled workflow
            println!("{}", (0..80).map(|_| '-').collect::<String>());
            ast::do_traversal(&wir, std::io::stdout()).unwrap();
            println!();

            // Analyse function calls (we'll assume this works too)
            let calls: HashMap<ProgramCounter, usize> =
                resolve_calls(&wir, &mut VirtualSymTable::with(&wir.table), &mut vec![], ProgramCounter::new(), None, None).unwrap().0;
            println!(
                "Resolved functions calls: {:?}",
                calls
                    .iter()
                    .map(|(pc, func_id)| (format!("{}", pc.display(&VirtualSymTable::with(&wir.table))), *func_id))
                    .collect::<HashMap<String, usize>>()
            );

            // Analyse the inlinable funcs
            let mut pred: HashMap<usize, bool> = HashMap::with_capacity(calls.len());
            find_inlinable_funcs(&wir, &calls, &mut VirtualSymTable::with(&wir.table), ProgramCounter::new(), None, &mut pred);
            println!("Inlinable functions: {pred:?}");
            println!();

            // Neat, done, assert it was right
            assert_eq!(pred, gold);
        }
    }
}





/***** ERRORS *****/
/// Defines errors that may occur when preprocessing a [`Workflow`].
#[derive(Debug)]
pub enum Error {
    /// Unknown task given.
    UnknownTask { id: usize },
    /// Unknown function given.
    UnknownFunc { id: usize },
    /// A [`Call`](ast::Edge::Call)-edge was encountered while we didn't know of a function ID on the stack.
    CallingWithoutId { pc: PrettyProgramCounter },
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Error::*;
        match self {
            UnknownTask { id } => write!(f, "Encountered unknown task ID {id} in Node"),
            UnknownFunc { id } => write!(f, "Encountered unknown function ID {id} in Call"),
            CallingWithoutId { pc } => write!(f, "Attempted to call function at {pc} without statically known task ID on the stack"),
        }
    }
}
impl error::Error for Error {}





/***** ANALYSIS FUNCTIONS *****/
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
fn pushes_func_id(instrs: &[EdgeInstr], idx: usize) -> Option<Option<usize>> {
    // Pop the next instruction
    let instr: &EdgeInstr = if idx < instrs.len() {
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
        EdgeInstr::Function { def } => Some(Some(*def)),

        // Things instructions only pop, potentially (accidentally) removing our function
        // Jep just tell the thign we don't know, we don't need it for direct function calls
        EdgeInstr::Pop {} | EdgeInstr::PopMarker {} | EdgeInstr::DynamicPop {} | EdgeInstr::VarSet { .. } => Some(None),

        // Alright some weird local branching; fuck it, also give up because we don't know which of the branches will do it
        EdgeInstr::Branch { .. } | EdgeInstr::BranchNot { .. } => Some(None),

        // These instructions never pop- or push anything
        EdgeInstr::VarDec { .. } | EdgeInstr::VarUndec { .. } => Some(None),

        // These instructions push invalid things _for sure_
        EdgeInstr::Cast { .. }
        | EdgeInstr::Not {}
        | EdgeInstr::Neg {}
        | EdgeInstr::And {}
        | EdgeInstr::Or {}
        | EdgeInstr::Add {}
        | EdgeInstr::Sub {}
        | EdgeInstr::Mul {}
        | EdgeInstr::Div {}
        | EdgeInstr::Mod {}
        | EdgeInstr::Eq {}
        | EdgeInstr::Ne {}
        | EdgeInstr::Lt {}
        | EdgeInstr::Le {}
        | EdgeInstr::Gt {}
        | EdgeInstr::Ge {}
        | EdgeInstr::Array { .. }
        | EdgeInstr::ArrayIndex { .. }
        | EdgeInstr::Instance { .. }
        | EdgeInstr::Proj { .. }
        | EdgeInstr::VarGet { .. }
        | EdgeInstr::Boolean { .. }
        | EdgeInstr::Integer { .. }
        | EdgeInstr::Real { .. }
        | EdgeInstr::String { .. } => Some(None),
    }
}

/// Analyses the edges in an [`Workflow`] to resolve function calls to the ID of the functions they call.
///
/// # Arguments
/// - `wir`: The [`Workflow`] to analyse.
/// - `table`: A running [`VirtualSymTable`] that determines the current types in scope.
/// - `trace`: A stack of call pointers that keeps track of the trace of function calls. Allows us to avoid recursion.
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
    wir: &Workflow,
    table: &mut VirtualSymTable,
    trace: &mut Vec<ProgramCounter>,
    pc: ProgramCounter,
    stack_id: Option<usize>,
    breakpoint: Option<ProgramCounter>,
) -> Result<(HashMap<ProgramCounter, usize>, Option<usize>), Error> {
    // Quit if we're at the breakpoint
    if let Some(breakpoint) = breakpoint {
        if pc == breakpoint {
            return Ok((HashMap::new(), None));
        }
    }

    // Get the edge in the workflow
    let edge: &Edge = match utils::get_edge(wir, pc) {
        Some(edge) => edge,
        None => return Ok((HashMap::new(), None)),
    };

    // Match to recursively process it
    trace!("Attempting to resolve calls in {} ({:?})", pc.display(table), edge.variant());
    match edge {
        Edge::Node { task, next, .. } => {
            // Attempt to discover the return type of the Node.
            let def: &TaskDef = match std::panic::catch_unwind(|| table.task(*task)) {
                Ok(def) => def,
                Err(_) => return Err(Error::UnknownTask { id: *task }),
            };

            // Alright, recurse with the next instruction
            resolve_calls(wir, table, trace, pc.jump(*next), if def.func().ret.is_void() { stack_id } else { None }, breakpoint)
        },

        Edge::Linear { instrs, next } => {
            // Analyse the instructions to find out if we can deduce a new `stack_id`
            let stack_id: Option<usize> = if !instrs.is_empty() { pushes_func_id(instrs, instrs.len() - 1).unwrap_or(stack_id) } else { stack_id };

            // Analyse the next one
            resolve_calls(wir, table, trace, pc.jump(*next), stack_id, breakpoint)
        },

        Edge::Stop {} => Ok((HashMap::new(), None)),

        Edge::Branch { true_next, false_next, merge } => {
            // First, analyse the branches
            let (mut calls, mut stack_id): (HashMap<_, _>, Option<usize>) =
                resolve_calls(wir, table, trace, pc.jump(*true_next), stack_id, merge.map(|merge| pc.jump(merge)))?;
            if let Some(false_next) = false_next {
                let (false_calls, false_stack) = resolve_calls(wir, table, trace, pc.jump(*false_next), stack_id, merge.map(|merge| pc.jump(merge)))?;
                calls.extend(false_calls);
                if stack_id != false_stack {
                    stack_id = None;
                }
            }

            // Analyse the remaining part next
            if let Some(merge) = merge {
                let (merge_calls, merge_stack) = resolve_calls(wir, table, trace, pc.jump(*merge), stack_id, breakpoint)?;
                calls.extend(merge_calls);
                stack_id = merge_stack;
            }

            // Alright, return the found results
            Ok((calls, stack_id))
        },

        Edge::Parallel { branches, merge } => {
            // Simply analyse all branches first. No need to worry about their return values and such, since that's not until the `Join`.
            let mut calls: HashMap<_, _> = HashMap::new();
            for branch in branches {
                calls.extend(resolve_calls(wir, table, trace, pc.jump(*branch), stack_id, breakpoint)?.0);
            }

            // OK, then analyse the rest assuming the stack is unchanged (we can do that because the parallel's branches get clones)
            let (new_calls, stack_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, trace, pc.jump(*merge), stack_id, breakpoint)?;
            calls.extend(new_calls);
            Ok((calls, stack_id))
        },

        Edge::Join { merge, next } => {
            // Simply do the next, only _not_ resetting the stack ID if no value is returned.
            resolve_calls(wir, table, trace, pc.jump(*next), if *merge == MergeStrategy::None { stack_id } else { None }, breakpoint)
        },

        Edge::Loop { cond, body, next } => {
            // Traverse the three individually, using the stack ID of the codebody that precedes it
            let (mut calls, mut cond_id): (HashMap<_, _>, Option<usize>) =
                resolve_calls(wir, table, trace, pc.jump(*cond), stack_id, Some(pc.jump(*body - 1)))?;
            let (body_calls, _): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, trace, pc.jump(*body), cond_id, Some(pc.jump(*cond)))?;
            calls.extend(body_calls);
            if let Some(next) = next {
                let (next_calls, next_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, trace, pc.jump(*next), cond_id, breakpoint)?;
                calls.extend(next_calls);
                cond_id = next_id;
            }

            // Done!
            Ok((calls, cond_id))
        },

        Edge::Call { input: _, result: _, next } => {
            // Alright time to jump functions based on the current top-of-the-stack
            let stack_id: usize = match stack_id {
                Some(id) => id,
                None => {
                    return Err(Error::CallingWithoutId { pc: pc.display(table) });
                },
            };

            // We can early quit upon recursion
            if trace.contains(&pc) {
                let mut calls: HashMap<ProgramCounter, usize> = HashMap::from([(pc, stack_id)]);
                let (next_calls, next_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, trace, pc.jump(*next), None, breakpoint)?;
                calls.extend(next_calls);
                return Ok((calls, next_id));
            }

            // Get the function definition to extend the VirtualSymTable
            let def: &FunctionDef = match catch_unwind(|| table.func(stack_id)) {
                Ok(def) => def,
                Err(_) => return Err(Error::UnknownFunc { id: stack_id }),
            };

            // Add the mapping to the table
            let mut calls: HashMap<ProgramCounter, usize> = HashMap::from([(pc, stack_id)]);

            // Resolve the call of the function (builtins simply return nothing, so are implicitly handled)
            table.push(&def.table);
            trace.push(pc);
            let (call_calls, call_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, trace, ProgramCounter::call(stack_id), None, None)?;
            trace.pop();
            table.pop();
            calls.extend(call_calls);

            // Then continue with the next one
            let (next_calls, next_id): (HashMap<_, _>, Option<usize>) = resolve_calls(wir, table, trace, pc.jump(*next), call_id, breakpoint)?;
            calls.extend(next_calls);
            Ok((calls, next_id))
        },

        Edge::Return { result: _ } => {
            // If we're in the main function, this acts as an [`Elem::Stop`] with value
            if pc.0 == usize::MAX {
                return Ok((HashMap::new(), None));
            }

            // To see whether we pass a function ID, consult the function definition
            let def: &FunctionDef = match catch_unwind(|| table.func(pc.0)) {
                Ok(def) => def,
                Err(_) => return Err(Error::UnknownFunc { id: pc.0 }),
            };

            // Only return the current one if the function returns void
            if def.ret.is_void() { Ok((HashMap::new(), stack_id)) } else { Ok((HashMap::new(), None)) }
        },
    }
}

/// Attempts to find all non-recursive functions in the given WIR.
///
/// The only moment when we don't consider a function inlinable is if the function call is:
/// - Recursive
/// - A builtin
/// - Undecidable
///
/// # Arguments
/// - `wir`: The input [WIR](Workflow) to analyse.
/// - `calls`: The map of call indices to which function is actually called.
/// - `table`: The [`VirtualSymTable`] keeping track of current definitions in scope.
/// - `pc`: Points to the current [`Edge`] to analyse.
/// - `breakpoint`: If given, then analysis should stop when this PC is hit.
/// - `inlinable`: The result we're recursively building. This set simply collects all function IDs and maps them to inlinable or not.
///
/// # Returns
/// A [`HashSet`] with all the IDs of the functions that are candidates for inlining.
fn find_inlinable_funcs(
    wir: &Workflow,
    calls: &HashMap<ProgramCounter, usize>,
    table: &mut VirtualSymTable,
    pc: ProgramCounter,
    breakpoint: Option<ProgramCounter>,
    inlinable: &mut HashMap<usize, bool>,
) {
    // Attempt to get the edge
    let edge: &Edge = match utils::get_edge(wir, pc) {
        Some(edge) => edge,
        None => return,
    };

    // Match on its kind
    match edge {
        Edge::Node { next, .. } | Edge::Linear { next, .. } => {
            // Doesn't call any functions, so just proceed with the next one
            find_inlinable_funcs(wir, calls, table, pc.jump(*next), breakpoint, inlinable);
        },

        Edge::Stop {} => return,

        Edge::Branch { true_next, false_next, merge } => {
            // Analyse the left branch...
            find_inlinable_funcs(wir, calls, table, pc.jump(*true_next), merge.map(|merge| pc.jump(merge)), inlinable);
            // ...the right branch...
            if let Some(false_next) = false_next {
                find_inlinable_funcs(wir, calls, table, pc.jump(*false_next), merge.map(|merge| pc.jump(merge)), inlinable);
            }
            // ...and the merge!
            if let Some(merge) = merge {
                find_inlinable_funcs(wir, calls, table, pc.jump(*merge), breakpoint, inlinable);
            }
        },

        Edge::Parallel { branches, merge } => {
            // Collect all the branches
            for branch in branches {
                find_inlinable_funcs(wir, calls, table, pc.jump(*branch), Some(pc.jump(*merge)), inlinable);
            }

            // Run merge and done is Cees
            find_inlinable_funcs(wir, calls, table, pc.jump(*merge), breakpoint, inlinable);
        },

        Edge::Join { next, .. } => find_inlinable_funcs(wir, calls, table, pc.jump(*next), breakpoint, inlinable),

        Edge::Loop { cond, body, next } => {
            // Traverse the condition...
            find_inlinable_funcs(wir, calls, table, pc.jump(*cond), Some(pc.jump(*body - 1)), inlinable);
            // ...the body...
            find_inlinable_funcs(wir, calls, table, pc.jump(*body), Some(pc.jump(*cond)), inlinable);
            // ...and finally, the next step, if any
            if let Some(next) = next {
                find_inlinable_funcs(wir, calls, table, pc.jump(*next), breakpoint, inlinable);
            }
        },

        Edge::Call { next, .. } => {
            // OK, the exciting point!

            // Resolve the function ID we're calling
            let func_id: usize = match calls.get(&pc) {
                Some(id) => *id,
                None => {
                    panic!("Encountered unresolved call after running call analysis");
                },
            };
            let def: &FunctionDef = match catch_unwind(|| table.func(func_id)) {
                Ok(def) => def,
                Err(_) => panic!("Failed to get definition of function {func_id} after call analysis"),
            };

            // Analyse next, since all codepaths do this always
            find_inlinable_funcs(wir, calls, table, pc.jump(*next), None, inlinable);

            // Functions are not inlinable if builtins; if so, return
            if BuiltinFunctions::is_builtin(&def.name) {
                inlinable.insert(func_id, false);
                return;
            }

            // Examine if this call would introduce a recursive problem
            if inlinable.contains_key(&func_id) {
                // We've already seen this one! Change our mind about its inlinability
                inlinable.insert(func_id, false);
                // NOTE: No need to go into the call body, as we've done this the first time we saw it
                return;
            }

            // If we get this far, let's assume that we're inlinable for now and recurse into the body
            inlinable.insert(func_id, true);
            table.push(&def.table);
            find_inlinable_funcs(wir, calls, table, ProgramCounter::call(func_id), None, inlinable);
            table.pop();
        },

        Edge::Return { result: _ } => return,
    }
}





/***** SIMPLIFICATION FUNCTIONS *****/
/// Attempts to inline functions in the WIR as much as possible.
///
/// The only moment when we don't is if the function call is:
/// - Recursive
/// - A builtin
/// - Undecidable
///
/// # Arguments
/// - `wir`: The input [WIR](Workflow) to simply.
/// - `calls`: The map of call indices to which function is actually called.
///
/// # Returns
/// The same `wir` as given, but then optimized.
///
/// # Errors
/// This function may error if the input workflow is incoherent.
pub fn inline_functions(mut wir: Workflow, calls: &HashMap<ProgramCounter, usize>) -> Workflow {
    // Analyse which functions in the WIR are non-recursive
    let mut inlinable: HashMap<usize, bool> = HashMap::with_capacity(calls.len());
    find_inlinable_funcs(&wir, calls, &mut VirtualSymTable::with(&wir.table), ProgramCounter::new(), None, &mut inlinable);

    // Inline them (ez pz lemon squeezy; or difficult difficult lemon difficult?)


    // OK, we did all we could
    wir
}





/***** LIBRARY *****/
/// Simplifies the given WIR-workflow as much as possible to increase the compatability with checker workflows.
///
/// Most importantly, it:
/// - Attempts to inline functions as long as they're non-recursive (since functions are not supported)
///
/// # Arguments
/// - `wir`: The input [WIR](Workflow) to simply.
///
/// # Returns
/// A tuple of the same `wir` as given, but then optimized, and a mapping of (remaining) [`Edge::Call`]s to whatever function they actually map.
///
/// # Errors
/// This function may error if the input workflow is incoherent.
pub fn simplify(mut wir: Workflow) -> Result<(Workflow, HashMap<ProgramCounter, usize>), Error> {
    // Analyse call dependencies first
    let (calls, _): (HashMap<ProgramCounter, usize>, _) =
        resolve_calls(&wir, &mut VirtualSymTable::with(&wir.table), &mut vec![], ProgramCounter::new(), None, None)?;

    // Simplify functions as much as possible
    wir = inline_functions(wir, &calls);

    // Done!
    Ok((wir, calls))
}

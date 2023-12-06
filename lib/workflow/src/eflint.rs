//  EFLINT.rs
//    by Lut99
//
//  Created:
//    08 Nov 2023, 14:44:31
//  Last edited:
//    06 Dec 2023, 17:50:34
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines a compiler of the Checker Workflow to the eFLINT JSON
//!   Specification.
//

use eflint_json::spec::{ConstructorInput, Expression, ExpressionConstructorApp, ExpressionPrimitive, Phrase, PhraseCreate};
use enum_debug::EnumDebug as _;
use log::{trace, warn};

use crate::spec::{Elem, ElemBranch, ElemCommit, ElemLoop, ElemParallel, ElemTask, User, Workflow};


/***** HELPER MACROS *****/
/// Shorthand for creating an eFLINT JSON Specification true postulation.
macro_rules! create {
    ($inst:expr) => {
        Phrase::Create(PhraseCreate { operand: $inst })
    };
}

/// Shorthand for creating an eFLINT JSON Specification constructor application.
macro_rules! constr_app {
    ($id:expr $(, $args:expr)* $(,)?) => {
        Expression::ConstructorApp(ExpressionConstructorApp {
            identifier: ($id).into(),
            operands:   ConstructorInput::ArraySyntax(vec![ $($args),* ]),
        })
    };
}

/// Shorthand for creating an eFLINT JSON Specification string literal.
macro_rules! str_lit {
    ($val:expr) => {
        Expression::Primitive(ExpressionPrimitive::String(($val).into()))
    };
}





/***** HELPER FUNCTIONS *****/
/// Compiles the given [`Elem`] onwards to a series of eFLINT [`Phrase`]s.
///
/// # Arguments
/// - `elem`: The current [`Elem`] we're compiling.
/// - `wf_id`: The identifier/name of the workflow we're working with.
/// - `wf_user`: The identifier/name of the user who will see the workflow result.
/// - `phrases`: The list of eFLINT [`Phrase`]s we're compiling to.
fn compile_eflint(mut elem: &Elem, wf_id: &str, wf_user: &User, phrases: &mut Vec<Phrase>) {
    // Note we're doing a combination of actual recursion and looping, to minimize stack usage
    loop {
        trace!("Compiling {:?} to eFLINT", elem.variant());
        match elem {
            Elem::Task(ElemTask { id, name, package, version, input, output, location, metadata, next }) => {
                // Define a new task call and make it part of the workflow
                // ```eflint
                // +node(workflow(#wf_id), #id).
                // +task(node(workflow(#wf_id), #id)).
                // ```
                let node: Expression = constr_app!("node", constr_app!("workflow", str_lit!(wf_id)), str_lit!(id.clone()));
                phrases.push(create!(node.clone()));
                phrases.push(create!(constr_app!("task", node.clone())));

                // Link the code input
                // ```eflint
                // +node-input(#node, asset("#package-#version")).
                // +function(node-input(#node, asset("#package-#version")), #name).
                // ```
                let code_input: Expression =
                    constr_app!("node-input", node.clone(), constr_app!("asset", str_lit!(format!("{}-{}", package, version))));
                phrases.push(create!(code_input.clone()));
                phrases.push(create!(constr_app!("function", code_input.clone(), str_lit!(name.clone()))));

                // Add its inputs
                for i in input {
                    // Link this input to the task
                    // ```eflint
                    // +node-input(#node, asset(#i.name)).
                    // ```
                    let node_input: Expression = constr_app!("node-input", node.clone(), constr_app!("asset", str_lit!(i.name.clone())));
                    phrases.push(create!(node_input.clone()));

                    // Add where this dataset lives if we know that
                    if let Some(from) = &i.from {
                        // It's planned to be transferred from this location
                        // ```eflint
                        // +node-input-from(#node-input, domain(user(#from))).
                        // ```
                        phrases.push(create!(constr_app!(
                            "node-input-from",
                            node_input,
                            constr_app!("domain", constr_app!("user", str_lit!(from.clone())))
                        )));
                    } else if let Some(at) = location {
                        // It's present on the task's location
                        // ```eflint
                        // +node-input-from(#node-input, domain(user(#at))).
                        // ```
                        phrases.push(create!(constr_app!(
                            "node-input-from",
                            node_input,
                            constr_app!("domain", constr_app!("user", str_lit!(at.clone())))
                        )));
                    } else {
                        warn!("Encountered input dataset '{}' without transfer source in task '{}' as part of workflow '{}'", i.name, id, wf_id);
                    }
                }
                // Add the output, if any
                if let Some(o) = &output {
                    // ```eflint
                    // +node-output(#node, asset(#o.name)).
                    // ```
                    phrases.push(create!(constr_app!("node-output", node.clone(), constr_app!("asset", str_lit!(o.name.clone())))));
                }
                // Add the location of the task execution
                if let Some(at) = location {
                    // ```eflint
                    // +node-at(#node, domain(user(#at))).
                    // ```
                    phrases.push(create!(constr_app!("task-at", node.clone(), constr_app!("domain", constr_app!("user", str_lit!(at.clone()))))));
                } else {
                    warn!("Encountered unplanned task '{id}' part of workflow '{wf_id}'");
                }

                // Finally, add any task metadata
                for m in metadata {
                    // ```eflint
                    // +node-metadata(#node, metadata(tag(user(#m.owner), #m.tag), signature(user(#m.assigner), #m.signature)))).
                    // ```
                    phrases.push(create!(constr_app!(
                        "node-metadata",
                        node.clone(),
                        constr_app!(
                            "metadata",
                            constr_app!("tag", constr_app!("user", str_lit!(m.owner.clone())), str_lit!(m.tag.clone())),
                            constr_app!("signature", constr_app!("user", str_lit!(m.assigner.clone())), str_lit!(m.signature.clone())),
                        )
                    )));
                }

                // OK, move to the next
                elem = next;
            },
            Elem::Commit(ElemCommit { id, data_name, location, input, next }) => {
                // Add the commit task
                // ```eflint
                // +node(workflow(#wf_id), #id).
                // +commit(node(workflow(#wf_id), #id)).
                // ```
                let node: Expression = constr_app!("node", constr_app!("workflow", str_lit!(wf_id)), str_lit!(id.clone()));
                phrases.push(create!(node.clone()));
                phrases.push(create!(constr_app!("commit", node.clone())));

                // Add the commits it (possibly!) does
                for i in input {
                    // ```eflint
                    // +node-input(#node, asset(#i.name)).
                    // ```
                    let node_input: Expression = constr_app!("node-input", node.clone(), constr_app!("asset", str_lit!(i.name.clone())));
                    phrases.push(create!(node_input.clone()));

                    // Add where this dataset lives if we know that
                    if let Some(from) = &i.from {
                        // It's planned to be transferred from this location
                        // ```eflint
                        // +node-input-from(#node-input, domain(user(#from))).
                        // ```
                        phrases.push(create!(constr_app!(
                            "node-input-from",
                            node_input,
                            constr_app!("domain", constr_app!("user", str_lit!(from.clone())))
                        )));
                    } else {
                        warn!("Encountered input dataset '{}' without transfer source in commit '{}' as part of workflow '{}'", i.name, id, wf_id);
                    }
                }
                // Add the output of the node
                // ```eflint
                // +node-output(#node, asset(#data_name)).
                // +workflow-result(workflow(#wf_id), asset(#data_name)).
                // ```
                phrases.push(create!(constr_app!("node-output", node.clone(), constr_app!("asset", str_lit!(data_name.clone())))));
                phrases.push(create!(constr_app!(
                    "workflow-result",
                    constr_app!("workflow", str_lit!(wf_id)),
                    constr_app!("asset", str_lit!(data_name.clone()))
                )));

                // Add the location of this commit
                if let Some(location) = location {
                    // ```eflint
                    // +node-at(#node, domain(user(#at))).
                    // ```
                    phrases.push(create!(constr_app!("node-at", node, constr_app!("domain", constr_app!("user", str_lit!(location.clone()))))));
                }

                // Continue with the next
                elem = next;
            },

            Elem::Branch(ElemBranch { branches, next }) => {
                // Do the branches in sequence
                for branch in branches {
                    compile_eflint(branch, wf_id, wf_user, phrases);
                }
                // Continue with the next one
                elem = next;
            },
            Elem::Parallel(ElemParallel { branches, merge: _, next }) => {
                // Do the branches in sequence
                for branch in branches {
                    compile_eflint(branch, wf_id, wf_user, phrases);
                }
                // Continue with the next one
                elem = next;
            },
            Elem::Loop(ElemLoop { body: _, next }) => {
                warn!("Compilation from Elem::Loop to eFLINT is not yet implementated.");
                elem = next;
            },

            Elem::Next => return,
            Elem::Stop(results) => {
                // Mark the results as results of the workflow
                for r in results {
                    // ```eflint
                    // +workflow-result-recipient(workflow-result(workflow(#wf_id), asset(#r.name)), user(#wf_user.name)).
                    // ```
                    phrases.push(create!(constr_app!(
                        "workflow-result-recipient",
                        constr_app!("workflow-result", constr_app!("workflow", str_lit!(wf_id)), constr_app!("asset", str_lit!(r.name.clone()))),
                        constr_app!("user", str_lit!(wf_user.name.clone())),
                    )));
                }

                // Done
                return;
            },
        }
    }
}





/***** LIBRARY *****/
impl Workflow {
    /// Compiles the Workflow to a series of eFLINT phrases.
    ///
    /// Note that this only creates references to datasets, functions and users; any definition of them needs to be added separately.
    ///
    /// # Returns
    /// A series of eFLINT statements that represent this Workflow.
    pub fn to_eflint(&self) -> Vec<Phrase> {
        let mut phrases: Vec<Phrase> = vec![];

        // First, add the notion of the workflow as a whole
        phrases.push(create!(constr_app!("workflow", str_lit!(self.id.clone()))));
        // Add workflow metadata
        for m in &self.metadata {
            phrases.push(create!(constr_app!(
                "workflow-metadata",
                constr_app!("workflow", str_lit!(self.id.clone())),
                constr_app!(
                    "metadata",
                    constr_app!(
                        "metadata",
                        constr_app!("owner", constr_app!("user", str_lit!(m.owner.clone()))),
                        constr_app!("tag", str_lit!(m.tag.clone())),
                        constr_app!("assigner", constr_app!("user", str_lit!(m.assigner.clone()))),
                        constr_app!("signature", str_lit!(m.signature.clone()))
                    )
                )
            )));
        }

        // Compile the 'flow to a list of phrases
        compile_eflint(&self.start, &self.id, &self.user, &mut phrases);

        // Done!
        phrases
    }
}

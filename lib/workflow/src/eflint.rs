//  EFLINT.rs
//    by Lut99
//
//  Created:
//    08 Nov 2023, 14:44:31
//  Last edited:
//    29 Nov 2023, 14:25:36
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

use crate::spec::{Elem, ElemBranch, ElemCommit, ElemLoop, ElemParallel, ElemTask, Workflow};


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
/// - `phrases`: The list of eFLINT [`Phrase`]s we're compiling to.
fn compile_eflint(mut elem: &Elem, wf_id: &str, phrases: &mut Vec<Phrase>) {
    // Note we're doing a combination of actual recursion and looping, to minimize stack usage
    loop {
        trace!("Compiling {:?} to eFLINT", elem.variant());
        match elem {
            Elem::Task(ElemTask { id, name, package, version, input, output, location, metadata, next }) => {
                // Define a new task call and make it part of the workflow
                phrases.push(create!(constr_app!("task", str_lit!(id.clone()))));
                phrases.push(create!(constr_app!("task-in", constr_app!("workflow", str_lit!(wf_id)), constr_app!("task", str_lit!(id.clone())))));

                // Add the container
                phrases.push(create!(constr_app!(
                    "function",
                    constr_app!("task", str_lit!(id.clone())),
                    str_lit!(name.clone()),
                    constr_app!("dataset", str_lit!(format!("{}-{}", package, version)))
                )));
                // Add its inputs and outputs
                for i in input {
                    // Link this input to the task
                    phrases.push(create!(constr_app!(
                        "argument",
                        constr_app!("task", str_lit!(id.clone())),
                        constr_app!("dataset", str_lit!(i.name.clone()))
                    )));

                    // Add where this dataset lives if we know that
                    if let Some(from) = &i.from {
                        phrases.push(create!(constr_app!(
                            "data-at",
                            constr_app!("dataset", str_lit!(i.name.clone())),
                            constr_app!("user", str_lit!(from.clone()))
                        )));
                    } else if let Some(at) = location {
                        phrases.push(create!(constr_app!(
                            "data-at",
                            constr_app!("dataset", str_lit!(i.name.clone())),
                            constr_app!("user", str_lit!(at.clone()))
                        )));
                    }
                }
                // Add the output, if any
                if let Some(o) = &output {
                    phrases.push(create!(constr_app!(
                        "output",
                        constr_app!("task", str_lit!(id.clone())),
                        constr_app!("dataset", str_lit!(o.name.clone()))
                    )));
                }
                // Add the location of the task execution
                if let Some(at) = location {
                    phrases.push(create!(constr_app!(
                        "task-at",
                        constr_app!("task", str_lit!(id.clone())),
                        constr_app!("domain", constr_app!("user", str_lit!(at.clone())))
                    )));
                } else {
                    warn!("Encountered unplanned task '{id}' part of workflow '{wf_id}'");
                }

                // Finally, add any task metadata
                for m in metadata {
                    phrases.push(create!(constr_app!(
                        "task-metadata",
                        constr_app!("task", str_lit!(id.clone())),
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

                // OK, move to the next
                elem = next;
            },
            Elem::Commit(ElemCommit { id, data_name, input, next }) => {
                // Add the commit task
                phrases.push(create!(constr_app!("commit", str_lit!(id.clone()))));

                // Add the commits it (possibly!) does
                for i in input {
                    phrases.push(create!(constr_app!(
                        "commits",
                        constr_app!("commit", str_lit!(id.clone())),
                        constr_app!("dataset", str_lit!(i.name.clone())),
                        constr_app!("dataset", str_lit!(data_name.clone()))
                    )));
                }

                // Continue with the next
                elem = next;
            },

            Elem::Branch(ElemBranch { branches: _, next }) => {
                warn!("Compilation from Elem::Branch to eFLINT is not yet implementated.");
                elem = next;
            },
            Elem::Parallel(ElemParallel { branches: _, merge: _, next }) => {
                warn!("Compilation from Elem::Parallel to eFLINT is not yet implementated.");
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
                    phrases.push(create!(constr_app!(
                        "result",
                        constr_app!("workflow", str_lit!(wf_id)),
                        constr_app!("dataset", str_lit!(r.name.clone()))
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
        compile_eflint(&self.start, &self.id, &mut phrases);

        // Done!
        phrases
    }
}

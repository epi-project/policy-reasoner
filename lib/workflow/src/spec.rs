//  SPEC.rs
//    by Lut99
//
//  Created:
//    27 Oct 2023, 15:56:55
//  Last edited:
//    13 Dec 2023, 08:37:20
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the checker workflow itself.
//

use std::collections::HashSet;
use std::hash::Hash;

use brane_ast::MergeStrategy;
use brane_ast::locations::Location;
use enum_debug::EnumDebug;
use serde::{Deserialize, Serialize};
use specifications::version::Version;

/***** AUXILLARY DATA *****/
/// Defines how a user looks like.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    /// The name of the user.
    pub name: String,
}

/// Defines a representation of a dataset.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Dataset {
    /// The name of the dataset.
    pub name: String,
    /// The place that we get it from. No transfer is necessary if this is the place of task execution.
    pub from: Option<Location>,
}
impl Dataset {
    /// Constructor for the Dataset that only takes information originating from the workflow.
    ///
    /// # Arguments
    /// - `name`: The name of the dataset.
    /// - `from`: The location where to pull the dataset from, or [`None`] if no transfer is planned (i.e., it lives on the same domain as the task using it as input).
    ///
    /// # Returns
    /// A new instance of self with the given `name` and `from`, and all other properties initialized to some default value.
    #[inline]
    pub fn new(name: impl Into<String>, from: impl Into<Option<Location>>) -> Self { Self { name: name.into(), from: from.into() } }
}
impl Eq for Dataset {}
impl PartialEq for Dataset {
    #[inline]
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}
impl Hash for Dataset {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.name.hash(state) }
}

/// Represents a "tag" and everything we need to know.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metadata {
    /// The "namespace" where the tag may be found. Represents the "owner", or the "definer" of the tag.
    pub owner: String,
    /// The tag itself.
    pub tag: String,
    /// The signature verifying this metadata. If present, it's given as a pair of the person signing it and their signature.
    pub signature: Option<(String, String)>,
}

/***** LIBRARY *****/
/// Defines the workflow's toplevel view.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Workflow {
    /// The identifier of this workflow as a whole.
    pub id:    String,
    /// Defines the first node in the workflow.
    pub start: Elem,

    /// The user instigating this workflow (and getting the result, if any).
    pub user:      User,
    /// The metadata associated with this workflow as a whole.
    pub metadata:  Vec<Metadata>,
    /// The signature verifying this workflow.
    pub signature: String,
}

/// Defines an element in the graph. This is either a _Node_, which defines a task execution, or an _Edge_, which defines how next tasks may be reached.
#[derive(Clone, Debug, Deserialize, EnumDebug, Serialize)]
pub enum Elem {
    // Nodes
    /// Defines a task that is executed, accessing and potentially producing data.
    Task(ElemTask),
    /// Defines the commiting of a result into a dataset, which will linger beyong the workflow with a specific name.
    Commit(ElemCommit),

    // Edges
    /// Defines an edge that connects to multiple next graph-branches of which only _one_ must be taken. Note that, because we don't include dynamic control flow information, we don't know _which_ will be taken.
    Branch(ElemBranch),
    /// Defines an edge that connects to multiple next graph-branches of which _all_ must be taken _concurrently_.
    Parallel(ElemParallel),
    /// Defines an edge that repeats a particular branch an unknown amount of times.
    Loop(ElemLoop),

    // Terminators
    /// Defines that the next element to execute is given by the parent `next`-field.
    Next,
    /// Defines that no more execution takes place.
    ///
    /// The option indicates if any data is carried to the remaining code.
    Stop(HashSet<Dataset>),
}

/// Defines a task node in the graph consisting of [`Elem`]s, which defines data access.
///
/// Yeah so basically represents a task execution, with all checker-relevant information.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElemTask {
    /// Some identifier for this call specifically.
    pub id: String,

    /// The name of the task to execute
    pub name:    String,
    /// The name of the package in which to find the task.
    pub package: String,
    /// The version number of the package in which to find the task.
    pub version: Version,

    /// Any input datasets used by the task.
    ///
    /// Note that this denotes a set of **possible** input sets. One or more of these may actually be used at runtime.
    pub input:  Vec<Dataset>,
    /// If there is an output dataset produced by this task, this names it.
    pub output: Option<Dataset>,

    /// The location where the task is planned to be executed, if any.
    pub location: Option<Location>,
    /// The list of metadata belonging to this task. Note: may need to be populated by the checker!
    pub metadata: Vec<Metadata>,

    /// The next graph element that this task connects to.
    pub next: Box<Elem>,
}

/// Defines a commit node in the graph consisting of [`Elem`]s, which defines data promotion.
///
/// Checkers can assume that anything produced by a function will be deleted after the workflow stops (or at least, domains **should** do so) _unless_ committed.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElemCommit {
    /// Some identifier for this call specifically.
    pub id: String,

    /// The name after committing.
    pub data_name: String,
    /// The location where the commit is planned to be "executed", if any.
    ///
    /// Note that this location is a little bit weird in the context of a commit, as it's just an adminstrative procedure. It can thus be interpreted purely as: "the location where the new output will be advertised".
    pub location:  Option<Location>,
    /// Any input datasets used by the task.
    ///
    /// Note that this denotes a set of **possible** input sets. One or more of these may actually be used at runtime.
    pub input:     Vec<Dataset>,

    /// The next graph element that this task connects to.
    pub next: Box<Elem>,
}

/// Defines a branching connection between graph [`Elem`]ents.
///
/// Or rather, defines a linear connection between two nodes, with a set of branches in between them.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElemBranch {
    /// The branches of which one _must_ be taken, but we don't know which one.
    pub branches: Vec<Elem>,
    /// The next graph element that this branching edge connects to.
    pub next:     Box<Elem>,
}

/// Defines a parallel connection between graph [`Elem`]ents.
///
/// Is like a [branch](ElemBranch), except that _all_ branches are taken _concurrently_ instead of only one.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElemParallel {
    /// The branches, _all_ of which but be taken _concurrently_.
    pub branches: Vec<Elem>,
    /// The method of joining the branches.
    pub merge:    MergeStrategy,
    /// The next graph element that this parallel edge connects to.
    pub next:     Box<Elem>,
}

/// Defines a looping connection between graph [`Elem`]ents.
///
/// Simply defines a branch that is taken repeatedly. Any condition that was there is embedded in the branching part, since that's how the branch is dynamically taken and we can't know how often any of them is taken anyway.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElemLoop {
    /// The body (and embedded condition) of the loop.
    pub body: Box<Elem>,
    /// The next graph element that this parallel edge connects to.
    pub next: Box<Elem>,
}

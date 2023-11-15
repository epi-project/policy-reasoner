use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum Input {
    #[serde(rename = "ping")]
    Ping(PingInput),
    #[serde(rename = "handshake")]
    Handshake(HandshakeInput),
    #[serde(rename = "phrases")]
    Phrases(PhrasesInput)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PingInput {
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HandshakeInput {
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PhrasesInput {
    pub version: String,
    pub phrases: Vec<Phrase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updates: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Phrase {
    Query(Query),
    Fact(Fact),
    Statement(Statement),
    Definition(Definition),
    ExtendEnum(ExtendEnum)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum Query {
    #[serde(rename = "bquery")]
    BQuery(BQuery),
    #[serde(rename = "iquery")]
    IQurey(IQuery),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct BQuery {
    pub expression: Expression,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct IQuery {
    pub expression: Expression,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_holds: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum Statement {
    #[serde(rename = "create")]
    Create(Create),
    #[serde(rename = "terminate")]
    Terminate(Terminate),
    #[serde(rename = "obfuscate")]
    Obfuscate(Obfuscate),
    #[serde(rename = "trigger")]
    Trigger(Trigger),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Create {
    pub operand: Expression,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Terminate {
    pub operand: Expression,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Obfuscate {
    pub operand: Expression,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trigger {
    pub operand: Expression,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum Fact {
    #[serde(rename = "afact")]
    AFact(AtomicFact),
    #[serde(rename = "cfact")]
    CFact(CompositeFact),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AtomicFact {
    pub name: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Vec<Expression>>,
    #[serde(rename = "derived-from", skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<Vec<Expression>>,
    #[serde(rename = "holds-when", skip_serializing_if = "Option::is_none")]
    pub holds_when: Option<Vec<Expression>>,
    #[serde(rename = "conditioned-by", skip_serializing_if = "Option::is_none")]
    pub conditioned_by: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_invariant: Option<bool>,
}

impl AtomicFact {
    pub fn new(name: String) -> Self {
        return Self { name: name, t: None, range: None, derived_from: None, holds_when: None, conditioned_by: None, is_invariant: None }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct CompositeFact {
    pub name: String,
    pub identified_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holds_when: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditioned_by: Option<Vec<Expression>>,
}

impl CompositeFact {
    pub fn new(name: String, identified_by: Vec<String>) -> Self {
        return Self { name, identified_by, derived_from: None, holds_when: None, conditioned_by: None }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum Definition {
    #[serde(rename = "placeholer")]
    Placeholder(Placeholder),
    #[serde(rename = "predicate")]
    Predicate(Predicate),
    #[serde(rename = "event")]
    Event(Event),
    #[serde(rename = "act")]
    Act(Act),
    #[serde(rename = "duty")]
    Duty(Duty),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Placeholder {
    pub name: Vec<String>,
    #[serde(rename = "for")]
    pub f: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Predicate {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_invariant: Option<bool>,
    pub expression: Expression,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Event {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holds_when: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditioned_by: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syncs_with: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creates: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminates: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfuscates: Option<Vec<Expression>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Act {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holds_when: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditioned_by: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syncs_with: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creates: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminates: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfuscates: Option<Vec<Expression>>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Duty {
    pub name: String,
    pub holder: String,
    pub claimant: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holds_when: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditioned_by: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violated_when: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforced_by: Option<Vec<Expression>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum ExtendEnum {
    #[serde(rename = "extend")]
    Extend(Extend)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Extend {
    pub name: String,
    #[serde(flatten)]
    pub extendable: Extendable,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag="parent-kind")]
pub enum Extendable {
    #[serde(rename = "duty")]
    DutyExtension(DutyExtension)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct DutyExtension {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holds_when: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditioned_by: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violated_when: Option<Vec<Expression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforced_by: Option<Vec<Expression>>,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Expression {
    Primitive(Primitive),
    VariableReference([String; 1]),
    ConstructorApplication(ConstructorApplication),
    Operator(Operator),
    Iterator(Iterator),
    Projection(Projection)
}



#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Primitive {
    Number(i64),
    String(String),
    Bool(bool)
}

// - Constructor applications
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConstructorApplication {
    pub identifier: String,
    pub operands: Vec<Expression>,
}

// - Operators
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Operator {
    pub operator: String,
    pub operands: Vec<Expression>,
}

// - Iterators
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Iterator {
    pub iterator: String,
    pub binds: Vec<String>,
    pub expression: Box<Expression>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Projection {
    pub parameter: String,
    pub operand: Box<Expression>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TriggerType {
    pub identifier: String,
    pub kind: String,
    pub parent: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Violation {
    pub kind: String,
    pub identifier: String,
    pub operands: Vec<Expression>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Error {
    pub id: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Result {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<Error>>,
    #[serde(flatten)]
    results: ResultTypes
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ResultTypes {
    Handshake(HandshakeResult),
    PhraseResult(PhraseResult),
    Ping(PingResult),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PingResult {}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct HandshakeResult {
    supported_versions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoner_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shares_updates: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shares_triggers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shares_violations: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum PhraseResult {
    BQueryResult(BQueryResult),
    IQueryResult(IQueryResult),
    StateChanges(StateChangeResult)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BQueryResult {
    result: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IQueryResult {
    results: Vec<ConstructorApplication>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StateChangeResult {
    results: Vec<StateChange>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StateChange {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    changes: Option<Vec<Phrase>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    triggers: Option<Vec<TriggerType>>,
    violated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    violations: Option<Vec<Violation>>,
}
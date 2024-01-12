use std::collections::HashMap;

use audit_logger::{ReasonerConnectorAuditLogger, SessionedConnectorAuditLogger};
use eflint_json::spec::auxillary::Version;
use eflint_json::spec::{
    ConstructorInput, Expression, ExpressionConstructorApp, ExpressionPrimitive, Phrase, PhraseCreate, Request, RequestCommon, RequestPhrases,
};
use log::{debug, error, info};
use policy::{Policy, PolicyContent};
use reasonerconn::{ReasonerConnError, ReasonerConnector, ReasonerConnectorFullContext, ReasonerResponse};
use state_resolver::State;
use workflow::spec::Workflow;


/***** HELPER MACROS *****/
/// Shortcut for creating an eFLINT JSON Specification [`Phrase::Create`].
///
/// # Arguments
/// - `inst`: A single eFLINT [`Expression`] that is an instance expression determining what to create; i.e., `foo(Amy, Bob)` in `+foo(Amy, Bob).`.
///
/// # Returns
/// A new [`Phrase::Create`] (or rather, the Rust code to create it).
macro_rules! create {
    ($inst:expr) => {
        Phrase::Create(PhraseCreate { operand: $inst })
    };
}

/// Shortcut for creating an eFLINT JSON Specification [`Expression::ConstructorApp`].
///
/// # Arguments
/// - _array syntax_
///   - `id`: The (string) identifier of the relation to construct; i.e., `foo` in `foo(Amy, Bob)`.
///   - `args...`: Zero or more addinitional [`Expression`]s that make up the arguments of the constructor application; i.e., `Amy` or `Bob` in `foo(Amy, Bob)`.
///
/// # Returns
/// A new [`Expression::ConstructorApp`] (or rather, the Rust code to create it).
macro_rules! constr_app {
    ($id:expr $(, $args:expr)* $(,)?) => {
        Expression::ConstructorApp(ExpressionConstructorApp {
            identifier: ($id).into(),
            operands:   ConstructorInput::ArraySyntax(vec![ $($args),* ]),
        })
    };
}

/// Shortcut for creating an eFLINT JSON Specification [`Expression::Primitive(ExpressionPrimitive::String)`].
///
/// # Arguments
/// - `val`: The string value to put in the string primitive. Note that this is automatically `into()`d; so passing a `&str` will work, for example.
///
/// # Returns
/// A new [`Expression::Primitive(ExpressionPrimitive::String)`] (or rather, the Rust code to create it).
macro_rules! str_lit {
    ($val:expr) => {
        Expression::Primitive(ExpressionPrimitive::String(($val).into()))
    };
}





/***** CONSTANTS *****/
/// The identifier used for this connector backend.
pub const EFLINT_JSON_ID: &'static str = "eflint-json";

// Externalized "constants"
/// The entire base specification, already serialized as eFLINT JSON. See `build.rs` to find how the `BASE_DEFS_EFLINT_JSON` environment variable is populated.
const JSON_BASE_SPEC: &str = include_str!(env!("BASE_DEFS_EFLINT_JSON"));
/// A hash of the entire base specification, precomputed by `build.rs`.
const JSON_BASE_SPEC_HASH: &str = env!("BASE_DEFS_EFLINT_JSON_HASH");





/***** LIBRARY *****/
pub struct EFlintReasonerConnector {
    pub addr:  String,
    base_defs: Vec<Phrase>,
}

impl EFlintReasonerConnector {
    pub fn new(addr: String) -> Self {
        info!("Creating new EFlintReasonerConnector to '{addr}'");
        let base_defs: RequestPhrases = serde_json::from_str(JSON_BASE_SPEC).unwrap();
        EFlintReasonerConnector { addr, base_defs: base_defs.phrases }
    }

    fn conv_state_to_eflint(&self, state: State) -> Vec<Phrase> {
        debug!(
            "Serializing state of {} datasets, {} functions, {} locations and {} users to eFLINT phrases",
            state.datasets.len(),
            state.functions.len(),
            state.locations.len(),
            state.users.len()
        );
        let mut result: Vec<Phrase> = Vec::<Phrase>::new();

        for user in state.users.iter() {
            // ```eflint
            // +user(#user.name).
            // ```
            let user_constr: Expression = constr_app!("user", str_lit!(user.name.clone()));
            result.push(create!(user_constr.clone()));
        }
        let user_len: usize = result.len();
        debug!("Generated {} user phrases", user_len);

        for location in state.locations.iter() {
            // ```eflint
            // +user(#location.name).
            // +domain(user(#location.name))
            // ```
            let user_constr: Expression = constr_app!("user", str_lit!(location.name.clone()));
            result.push(create!(user_constr.clone()));
            result.push(create!(constr_app!("domain", user_constr)));

            // add metadata
        }
        let location_len: usize = result.len();
        debug!("Generated {} location phrases", location_len - user_len);

        for dataset in state.datasets.iter() {
            // ```eflint
            // +asset(#data.name).
            // ```
            result.push(create!(constr_app!("asset", str_lit!(dataset.name.clone()))));
        }
        let dataset_len: usize = result.len();
        debug!("Generated {} dataset phrases", dataset_len - location_len);

        for function in state.functions.iter() {
            // ```eflint
            // +asset(#function.name).
            // +code(asset(#function.name)).
            // ```
            let asset_constr: Expression = constr_app!("asset", str_lit!(function.name.clone()));
            result.push(create!(asset_constr.clone()));
            result.push(create!(constr_app!("code", asset_constr)));
        }
        let function_len: usize = result.len();
        debug!("Generated {} function phrases", function_len - dataset_len);

        return result;
    }

    fn extract_eflint_policy(&self, policy: &Policy) -> Vec<Phrase> {
        info!("Extracting eFLINT policy...");
        let eflint_content: Vec<&PolicyContent> = policy.content.iter().filter(|x| x.reasoner == EFLINT_JSON_ID).collect();
        let eflint_content = eflint_content.first().unwrap();
        debug!("Deserializing input to eFLINT JSON...");
        let content: &str = eflint_content.content.get();
        let result: RequestPhrases = match serde_json::from_str(content) {
            Ok(result) => match result {
                Request::Phrases(phrases) => phrases,
                Request::Handshake(_) | Request::Inspect(_) | Request::Ping(_) => panic!("Cannot accept non-Phrases Request input from request"),
            },
            Err(err) => panic!(
                "Input is not valid eFLINT JSON: {err}\n\nInput:\n{}\n{}\n{}\n",
                (0..80).map(|_| '-').collect::<String>(),
                content,
                (0..80).map(|_| '-').collect::<String>()
            ),
        };
        result.phrases
    }

    fn conv_workflow(&self, workflow: Workflow) -> Vec<Phrase> {
        info!("Compiling Checker Workflow to eFLINT phrases...");
        workflow.to_eflint()
    }

    fn extract_eflint_version(&self, policy: &Policy) -> Result<Version, String> {
        info!("Retrieving eFLINT reasoner version from policy...");
        let eflint_content: Vec<&PolicyContent> = policy.content.iter().filter(|x| x.reasoner == EFLINT_JSON_ID).collect();
        let eflint_content = eflint_content.first().unwrap();
        let parts: Vec<&str> = eflint_content.reasoner_version.split(".").collect();

        if parts.len() != 3 {
            return Err(format!("Invalid version format, should be 'maj.min.patch', got '{}'", eflint_content.reasoner_version));
        }

        let maj = parts[0].parse::<u32>().map_err(|_| format!("Invalid major version part, could not parse {} into u32", parts[0]))?;
        let min = parts[1].parse::<u32>().map_err(|_| format!("Invalid minor version part, could not parse {} into u32", parts[1]))?;
        let patch = parts[2].parse::<u32>().map_err(|_| format!("Invalid patch version part, could not parse {} into u32", parts[2]))?;

        Ok(Version(maj, min, patch))
    }

    fn build_phrases(&self, policy: &Policy, state: State, workflow: Workflow, question: Phrase) -> Vec<Phrase> {
        let mut phrases = Vec::<Phrase>::new();

        // Build request
        // 1. Base Facts
        debug!("Loading interface ({} phrase(s))", self.base_defs.len());
        phrases.extend(self.base_defs.clone());

        // 2. Fill knowledgebase from state
        let state_phrases: Vec<Phrase> = self.conv_state_to_eflint(state);
        debug!("Loading state ({} phrase(s))", state_phrases.len());
        phrases.extend(state_phrases);

        // 3. Add request
        debug!("Loading question (1 phrase(s))");
        phrases.push(question);

        // 4. Add workflow
        let workflow_phrases: Vec<Phrase> = self.conv_workflow(workflow);
        debug!("Loading workflow ({} phrase(s))", workflow_phrases.len());
        phrases.extend(workflow_phrases);

        // 5. Add Policy
        let policy_phrases: Vec<Phrase> = self.extract_eflint_policy(&policy);
        debug!("Loading policy ({} phrase(s))", policy_phrases.len());
        phrases.extend(policy_phrases);

        phrases
    }

    async fn process_phrases<L: ReasonerConnectorAuditLogger + Send + Sync>(
        &self,
        logger: SessionedConnectorAuditLogger<L>,
        policy: &Policy,
        phrases: Vec<Phrase>,
    ) -> Result<ReasonerResponse, ReasonerConnError> {
        let version = self.extract_eflint_version(policy).map_err(|err| ReasonerConnError::new(err))?;
        debug!("Full request length: {} phrase(s)", phrases.len());
        let request = Request::Phrases(RequestPhrases { common: RequestCommon { version, extensions: HashMap::new() }, phrases, updates: true });
        debug!("Full request:\n\n{}\n\n", serde_json::to_string_pretty(&request).unwrap_or_else(|_| "<serialization failure>".into()));

        // Make request
        debug!("Sending eFLINT exec-task request to '{}'", self.addr);
        let client = reqwest::Client::new();
        let res = client.post(&self.addr).json(&request).send().await.map_err(|err| ReasonerConnError::new(err.to_string()))?;

        debug!("Awaiting response...");
        let raw_body = res.text().await.map_err(|err| ReasonerConnError::new(err.to_string()))?;

        debug!("Log raw response...");

        logger.log_reasoner_response(&raw_body).await.map_err(|err| {
            debug!("Error trying to log{:?}", err);
            ReasonerConnError::new(err.to_string())
        })?;

        debug!("Parsing response...");
        let response = serde_json::from_str::<eflint_json::spec::ResponsePhrases>(&raw_body).map_err(|err| {
            error!(
                "{}\n\nRaw response:\n{}\n{}\n{}\n",
                err,
                (0..80).map(|_| '-').collect::<String>(),
                raw_body,
                (0..80).map(|_| '-').collect::<String>()
            );
            ReasonerConnError::new(err.to_string())
        })?;

        debug!("Analysing response...");
        let _errors: Vec<String> = response
            .results
            .last()
            .map(|r| match r {
                eflint_json::spec::PhraseResult::StateChange(sc) => match &sc.violations {
                    Some(v) => v.iter().map(|v| v.identifier.clone()).collect(),
                    None => vec![],
                },
                _ => vec![],
            })
            .unwrap_or_else(Vec::new);

        // For now don't leak errors
        let errors: Vec<String> = Vec::new();

        // TODO proper handle invalid query and unexpected result
        let success: Result<bool, String> = response
            .results
            .last()
            .map(|r| match r {
                eflint_json::spec::PhraseResult::BooleanQuery(r) => Ok(r.result),
                eflint_json::spec::PhraseResult::InstanceQuery(_) => Err("Invalid query".into()),
                eflint_json::spec::PhraseResult::StateChange(r) => Ok(!r.violated),
            })
            .unwrap_or_else(|| Err("Unexpected result".into()));

        match success {
            Ok(success) => {
                debug!(
                    "Response judged as: {} ({} && {})",
                    if success && response.common.success { "success" } else { "violated" },
                    success,
                    response.common.success
                );
                Ok(ReasonerResponse::new(success && response.common.success, errors))
            },
            // TODO better error handling
            Err(err) => Err(ReasonerConnError::new(err)),
        }
    }
}


#[async_trait::async_trait]
impl<L: ReasonerConnectorAuditLogger + Send + Sync + 'static> ReasonerConnector<L> for EFlintReasonerConnector {
    type Context = &'static str;

    // type FullContext = reasonerconn::ReasonerConnectorFullContext;

    #[inline]
    fn context(&self) -> Self::Context { JSON_BASE_SPEC_HASH }

    #[inline]
    fn full_context(&self) -> ReasonerConnectorFullContext {
        ReasonerConnectorFullContext {
            name: "EFLINT connector".into(),
            t: EFLINT_JSON_ID.into(),
            version: "0.1.0".into(),
            base_defs: JSON_BASE_SPEC.into(),
            base_defs_hash: JSON_BASE_SPEC_HASH.into(),
        }
    }

    async fn execute_task(
        &self,
        logger: SessionedConnectorAuditLogger<L>,
        policy: Policy,
        state: State,
        workflow: Workflow,
        task: String,
    ) -> Result<ReasonerResponse, ReasonerConnError> {
        info!("Considering task '{}' in workflow '{}' for execution", task, workflow.id);

        // Add the question for this task
        // ```eflint
        // +task-to-execute(task(node(workflow(#workflow.id), #task))).
        // ```
        let question: Phrase = create!(constr_app!(
            "task-to-execute",
            constr_app!("task", constr_app!("node", constr_app!("workflow", str_lit!(workflow.id.clone())), str_lit!(task)))
        ));

        // Build & submit the phrases with the given policy, state, workflow _and_ question
        let phrases = self.build_phrases(&policy, state, workflow, question);
        self.process_phrases(logger, &policy, phrases).await
    }

    async fn access_data_request(
        &self,
        logger: SessionedConnectorAuditLogger<L>,
        policy: Policy,
        state: State,
        workflow: Workflow,
        data: String,
        task: Option<String>,
    ) -> Result<ReasonerResponse, ReasonerConnError> {
        // Determine if we're asking for a node-to-node data transfer (there's a task as context) or a node-to-user (there's no task).
        let question: Phrase = match task {
            Some(task_id) => {
                info!("Considering data access '{}' for task '{}' in workflow '{}'", data, task_id, workflow.id);

                // ```eflint
                // +dataset-to-transfer(node-input(node(workflow(#workflow.id), #task), asset(#data))).
                // ```
                create!(constr_app!(
                    "dataset-to-transfer",
                    constr_app!(
                        "node-input",
                        constr_app!("node", constr_app!("workflow", str_lit!(workflow.id.clone())), str_lit!(task_id)),
                        constr_app!("asset", str_lit!(data)),
                    )
                ))
            },
            None => {
                info!("Considering data access '{}' for result of workflow '{}'", data, workflow.id);

                // ```eflint
                // +result-to-transfer(workflow-result-recipient(workflow-result(workflow(#workflow.id), asset(#data)), user(#workflow.user))).
                // ```
                create!(constr_app!(
                    "result-to-transfer",
                    constr_app!(
                        "workflow-result-recipient",
                        constr_app!("workflow-result", constr_app!("workflow", str_lit!(workflow.id.clone())), constr_app!("asset", str_lit!(data))),
                        constr_app!("user", str_lit!(workflow.user.name.clone()))
                    )
                ))
            },
        };

        let phrases = self.build_phrases(&policy, state, workflow, question);
        self.process_phrases(logger, &policy, phrases).await
    }

    async fn workflow_validation_request(
        &self,
        logger: SessionedConnectorAuditLogger<L>,
        policy: Policy,
        state: State,
        workflow: Workflow,
    ) -> Result<ReasonerResponse, ReasonerConnError> {
        info!("Considering workflow '{}'", workflow.id);

        // Add the question for this task
        // ```eflint
        // +workflow-to-execute(workflow(#workflow.id)).
        // ```
        let question = create!(constr_app!("workflow-to-execute", constr_app!("workflow", str_lit!(workflow.id.clone()))));

        // Build & submit the phrases with the given policy, state, workflow _and_ question
        let phrases = self.build_phrases(&policy, state, workflow, question);
        self.process_phrases(logger, &policy, phrases).await
    }
}

use std::error::Error;
use std::iter::FlatMap;

use eflint_json::json::{AtomicFact, CompositeFact, ConstructorApplication, Create, Expression, Fact, Input, Phrase, PhrasesInput};
use log::{debug, info};
use policy::policy::{Policy, PolicyContent};
use state_resolver::State;
use workflow::spec::Workflow;

use crate::connector::{ReasonerConnError, ReasonerConnector, ReasonerResponse};

pub struct EFlintReasonerConnector {
    pub addr:  String,
    base_defs: Vec<Phrase>,
}

static JSON_BASE_SPEC: &str = include_str!("./base-defs.eflint.json");

impl EFlintReasonerConnector {
    pub fn new(addr: String) -> Self {
        info!("Creating new EFlintReasonerConnector to '{addr}'");
        let base_defs: Vec<Phrase> = serde_json::from_str(JSON_BASE_SPEC).unwrap();
        EFlintReasonerConnector { addr, base_defs }
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
            // Create users
            result.push(Phrase::Statement(eflint_json::json::Statement::Create(Create {
                operand: Expression::ConstructorApplication(ConstructorApplication {
                    identifier: "user".into(),
                    operands:   vec![Expression::Primitive(eflint_json::json::Primitive::String(user.name.clone().into()))],
                }),
            })));
        }
        let user_len: usize = result.len();
        debug!("Generated {} user phrases", user_len);

        for location in state.locations.iter() {
            // Create users
            result.push(Phrase::Statement(eflint_json::json::Statement::Create(Create {
                operand: Expression::ConstructorApplication(ConstructorApplication {
                    identifier: "user".into(),
                    operands:   vec![Expression::Primitive(eflint_json::json::Primitive::String(location.name.clone().into()))],
                }),
            })));

            // Create domains
            result.push(Phrase::Statement(eflint_json::json::Statement::Create(Create {
                operand: Expression::ConstructorApplication(ConstructorApplication {
                    identifier: "domain".into(),
                    operands:   vec![Expression::Primitive(eflint_json::json::Primitive::String(location.name.clone().into()))],
                }),
            })));

            // add metadata
        }
        let location_len: usize = result.len();
        debug!("Generated {} location phrases", location_len - user_len);

        for dataset in state.datasets.iter() {
            result.push(Phrase::Fact(Fact::AFact(AtomicFact::new(dataset.name.clone()))))
            // TODO other props
        }
        let dataset_len: usize = result.len();
        debug!("Generated {} dataset phrases", dataset_len - location_len);

        for function in state.functions.iter() {
            result.push(Phrase::Fact(Fact::AFact(AtomicFact::new(function.name.clone()))))
            // TODO other props
        }
        let function_len: usize = result.len();
        debug!("Generated {} function phrases", function_len - dataset_len);

        return result;
    }

    fn extract_eflint_policy(&self, policy: &Policy) -> Vec<Phrase> {
        info!("Extracting eFLINT policy...");
        let eflint_content: Vec<&PolicyContent> = policy.content.iter().filter(|x| x.reasoner == "eflint").collect();
        let eflint_content = eflint_content.first().unwrap();
        debug!("Deserializing input to eFLINT JSON...");
        let result: Vec<Phrase> = serde_json::from_str(eflint_content.content.get()).unwrap();
        result
    }

    fn conv_workflow(&self, workflow: Workflow) -> Vec<Phrase> {
        info!("Compiling Checker Workflow to eFLINT phrases...");
        workflow.to_eflint()
    }

    fn extract_eflint_version(&self, policy: &Policy) -> String {
        info!("Retrieving eFLINT reasoner version from policy...");
        let eflint_content: Vec<&PolicyContent> = policy.content.iter().filter(|x| x.reasoner == "eflint").collect();
        let eflint_content = eflint_content.first().unwrap();
        eflint_content.reasoner_version.clone()
    }
}

#[async_trait::async_trait]
impl ReasonerConnector for EFlintReasonerConnector {
    async fn execute_task(
        &self,
        policy: Policy,
        state: State,
        workflow: Workflow,
        task: String,
    ) -> Result<ReasonerResponse, Box<dyn std::error::Error>> {
        info!("Considering task '{}' in workflow '{}' for execution", task, workflow.id);
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
        phrases.push(Phrase::Statement(eflint_json::json::Statement::Create(Create {
            operand: Expression::ConstructorApplication(ConstructorApplication {
                identifier: "task-to-execute".into(),
                operands:   vec![Expression::Primitive(eflint_json::json::Primitive::String(task))],
            }),
        })));

        // 4. Add workflow
        let workflow_phrases: Vec<Phrase> = self.conv_workflow(workflow);
        debug!("Loading workflow ({} phrase(s))", workflow_phrases.len());
        phrases.extend(workflow_phrases);

        // 5. Add Policy
        let policy_phrases: Vec<Phrase> = self.extract_eflint_policy(&policy);
        debug!("Loading policy ({} phrase(s))", policy_phrases.len());
        phrases.extend(policy_phrases);

        debug!("Full request:\n\n{}\n\n", serde_json::to_string_pretty(&phrases).unwrap_or_else(|_| "<serialization failure>".into()));
        debug!("Full request length: {} phrase(s)", phrases.len());
        let request = Input::Phrases(PhrasesInput { version: self.extract_eflint_version(&policy), phrases, updates: Some(true) });

        // Make request
        debug!("Sending eFLINT exec-task request to '{}'", self.addr);
        let client = reqwest::Client::new();
        let res = client.post(&self.addr).json(&request).send().await?;

        debug!("Awaiting response...");
        let response = res.json::<eflint_json::json::Result>().await?;

        debug!("Analysing response...");
        let errors: Vec<String> = match &response.results {
            eflint_json::json::ResultTypes::PhraseResult(r) => match r {
                eflint_json::json::PhraseResult::StateChanges(changes) => {
                    if changes.results.last().unwrap().violated {
                        let x: Vec<String> = changes.results.last().unwrap().violations.clone().unwrap().into_iter().map(|v| v.identifier).collect();
                        x
                    } else {
                        Vec::<String>::new()
                    }
                },
                _ => Vec::<String>::new(),
            },
            _ => Vec::<String>::new(),
        };

        let success = match &response.results {
            eflint_json::json::ResultTypes::PhraseResult(r) => match r {
                eflint_json::json::PhraseResult::StateChanges(changes) => !changes.results.last().unwrap().violated,
                _ => false,
            },
            _ => false,
        };
        debug!("Response judged as: {}", if success { "success" } else { "violated" });



        // let errors = match response.errors {
        //     Some(errors) => {
        //         errors.into_iter().map(|err| err.message).collect()
        //     },
        //     None => {
        //         Vec::<String>::new()
        //     }
        // };

        Ok(ReasonerResponse::new(success && response.success, errors))
    }

    async fn access_data_request(
        &self,
        policy: Policy,
        state: State,
        workflow: Workflow,
        data: String,
        task: Option<String>,
    ) -> Result<ReasonerResponse, Box<dyn Error>> {
        todo!()
    }

    async fn workflow_validation_request(&self, policy: Policy, state: State, workflow: Workflow) -> Result<ReasonerResponse, Box<dyn Error>> {
        todo!()
    }
}

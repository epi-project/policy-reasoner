use std::{error::Error, iter::FlatMap};
use policy::policy::{Policy, PolicyContent};
use workflow::spec::Workflow;

use crate::connector::{ReasonerConnector, ReasonerResponse, ReasonerConnError};
use state_resolver::{State};
use eflint_json::json::{Phrase, PhrasesInput, AtomicFact, Fact, Expression, Create, ConstructorApplication, CompositeFact, Input};

pub struct EFlintReasonerConnector {
    pub addr: String,
    base_defs: Vec<Phrase>
}

static JSON_BASE_SPEC: &str = include_str!("./base-defs.eflint.json");

impl EFlintReasonerConnector {
    pub fn new(addr: String) -> Self {
        let base_defs: Vec<Phrase> = serde_json::from_str(JSON_BASE_SPEC).unwrap();
        EFlintReasonerConnector{
            addr,
            base_defs,
        }
    }

    fn conv_state_to_eflint(&self, state: State) -> Vec<Phrase> {
        let mut result: Vec<Phrase> = Vec::<Phrase>::new();

        for user in state.users.iter() {
            // Create users
            result.push(Phrase::Statement(
                eflint_json::json::Statement::Create(
                    Create{
                        operand: Expression::ConstructorApplication(
                            ConstructorApplication{
                                identifier: "user".into(),
                                operands: vec![
                                    Expression::Primitive(
                                        eflint_json::json::Primitive::String(user.name.clone().into())
                                    )
                                ]
                            }
                        )
                    }
                )
            ));
        }

        for location in state.locations.iter() {
            // Create users
            result.push(Phrase::Statement(
                eflint_json::json::Statement::Create(
                    Create{
                        operand: Expression::ConstructorApplication(
                            ConstructorApplication{
                                identifier: "user".into(),
                                operands: vec![
                                    Expression::Primitive(
                                        eflint_json::json::Primitive::String(location.name.clone().into())
                                    )
                                ]
                            }
                        )
                    }
                )
            ));

            // Create domains
            result.push(Phrase::Statement(
                eflint_json::json::Statement::Create(
                    Create{
                        operand: Expression::ConstructorApplication(
                            ConstructorApplication{
                                identifier: "domain".into(),
                                operands: vec![
                                    Expression::Primitive(
                                        eflint_json::json::Primitive::String(location.name.clone().into())
                                    )
                                ]
                            }
                        )
                    }
                )
            ));

            // add metadata
        }

        for dataset in state.datasets.iter() {
            result.push(Phrase::Fact(Fact::AFact(AtomicFact::new(dataset.name.clone()))))
            // TODO other props
        }
        for function in state.functions.iter() {
            result.push(Phrase::Fact(Fact::AFact(AtomicFact::new(function.name.clone()))))
            // TODO other props
        }

        return result;
    }

    fn extract_eflint_policy(&self, policy: &Policy) -> Vec<Phrase> {
        let eflint_content : Vec<&PolicyContent> = policy.content.iter().filter(|x| x.reasoner == "eflint").collect();
        let eflint_content = eflint_content.first().unwrap();
        let result : Vec::<Phrase> = serde_json::from_str(eflint_content.content.get()).unwrap();
        result
    }

    fn conv_workflow(&self, workflow: Workflow) -> Vec<Phrase> {
        workflow.to_eflint()
    }

    fn extract_eflint_version(&self, policy: &Policy) -> String {
        let eflint_content : Vec<&PolicyContent> = policy.content.iter().filter(|x| x.reasoner == "eflint").collect();
        let eflint_content = eflint_content.first().unwrap();
        eflint_content.reasoner_version.clone()
    }

}

#[async_trait::async_trait]
impl ReasonerConnector for EFlintReasonerConnector {


    async fn execute_task(&self, policy: Policy, state: State, workflow: Workflow, task: String) -> Result<ReasonerResponse, Box<dyn std::error::Error>> {
        let mut phrases = Vec::<Phrase>::new();

        // Build request
        // 1. Base Facts
        phrases.extend(self.base_defs.clone());

        // 2. Fill knowledgebase from state
        phrases.extend(self.conv_state_to_eflint(state));

        // 3. Add request
        phrases.push(Phrase::Statement(
            eflint_json::json::Statement::Create(
                Create{
                    operand: Expression::ConstructorApplication(
                        ConstructorApplication{
                            identifier: "task-to-execute".into(),
                            operands: vec![
                                Expression::Primitive(
                                    eflint_json::json::Primitive::String(task)
                                )
                            ]
                        }
                    )
                }
            )
        ));

        // 4. Add workflow
        phrases.extend(self.conv_workflow(workflow));

        // 5. Add Policy
        phrases.extend(self.extract_eflint_policy(&policy));

        let request = Input::Phrases(PhrasesInput{
            version: self.extract_eflint_version(&policy),
            phrases,
            updates: Some(true),
        });

        // Make request
        let client = reqwest::Client::new();
        let res = client.post(&self.addr)
            .json(&request)
            .send().await?;

        let response = res.json::<eflint_json::json::Result>().await?;

        let errors: Vec<String> = match &response.results {
            eflint_json::json::ResultTypes::PhraseResult(r) => match r {
                eflint_json::json::PhraseResult::StateChanges(changes) => {
                    if changes.results.last().unwrap().violated {
                        let x : Vec<String> = changes.results.last().unwrap().violations.clone().unwrap().into_iter().map(|v| v.identifier).collect();
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

    async fn access_data_request(&self, policy: Policy, state: State, workflow: Workflow, data: String, task: Option<String>) -> Result<ReasonerResponse, Box<dyn Error>> {
        todo!()
    }

    async fn workflow_validation_request(&self, policy: Policy, state: State, workflow: Workflow) -> Result<ReasonerResponse, Box<dyn Error>> {
        todo!()
    }
}


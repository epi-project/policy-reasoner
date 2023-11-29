use std::collections::HashMap;
use std::error::Error;

use eflint_json::spec::auxillary::{AtomicType, Version};
use eflint_json::spec::{
    ConstructorInput, Expression, ExpressionConstructorApp, ExpressionPrimitive, Phrase, PhraseAtomicFact, PhraseCreate, RequestCommon,
    RequestPhrases, TypeDefinitionCommon,
};
use log::{debug, info};
use policy::{Policy, PolicyContent};
use reasonerconn::{ReasonerConnector, ReasonerResponse};
use state_resolver::State;
use workflow::spec::Workflow;

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
            // PhraseCreate users
            result.push(Phrase::Create(PhraseCreate {
                operand: Expression::ConstructorApp(ExpressionConstructorApp {
                    identifier: "user".into(),
                    operands:   ConstructorInput::ArraySyntax(vec![Expression::Primitive(ExpressionPrimitive::String(user.name.clone().into()))]),
                }),
            }));
        }
        let user_len: usize = result.len();
        debug!("Generated {} user phrases", user_len);

        for location in state.locations.iter() {
            // PhraseCreate users
            result.push(Phrase::Create(PhraseCreate {
                operand: Expression::ConstructorApp(ExpressionConstructorApp {
                    identifier: "user".into(),
                    operands:   ConstructorInput::ArraySyntax(vec![Expression::Primitive(ExpressionPrimitive::String(location.name.clone().into()))]),
                }),
            }));

            // PhraseCreate domains
            result.push(Phrase::Create(PhraseCreate {
                operand: Expression::ConstructorApp(ExpressionConstructorApp {
                    identifier: "domain".into(),
                    operands:   ConstructorInput::ArraySyntax(vec![Expression::Primitive(ExpressionPrimitive::String(location.name.clone().into()))]),
                }),
            }));

            // add metadata
        }
        let location_len: usize = result.len();
        debug!("Generated {} location phrases", location_len - user_len);

        for dataset in state.datasets.iter() {
            result.push(Phrase::AtomicFact(PhraseAtomicFact {
                name: dataset.name.clone(),
                ty: AtomicType::String,
                definition: TypeDefinitionCommon { derived_from: vec![], holds_when: vec![], conditioned_by: vec![] },
                range: None,
            }))
        }
        let dataset_len: usize = result.len();
        debug!("Generated {} dataset phrases", dataset_len - location_len);

        for function in state.functions.iter() {
            result.push(Phrase::AtomicFact(PhraseAtomicFact {
                name: function.name.clone(),
                ty: AtomicType::String,
                definition: TypeDefinitionCommon { derived_from: vec![], holds_when: vec![], conditioned_by: vec![] },
                range: None,
            }))
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

    fn extract_eflint_version(&self, policy: &Policy) -> Result<Version, String> {
        info!("Retrieving eFLINT reasoner version from policy...");
        let eflint_content: Vec<&PolicyContent> = policy.content.iter().filter(|x| x.reasoner == "eflint").collect();
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

    async fn process_phrases(&self, policy: &Policy, phrases: Vec<Phrase>) -> Result<ReasonerResponse, Box<dyn std::error::Error>> {
        debug!("Full request:\n\n{}\n\n", serde_json::to_string_pretty(&phrases).unwrap_or_else(|_| "<serialization failure>".into()));
        debug!("Full request length: {} phrase(s)", phrases.len());
        let version = self.extract_eflint_version(policy)?;
        let request = RequestPhrases { common: RequestCommon { version, extensions: HashMap::new() }, phrases, updates: true };



        // Make request
        debug!("Sending eFLINT exec-task request to '{}'", self.addr);
        let client = reqwest::Client::new();
        let res = client.post(&self.addr).json(&request).send().await?;

        debug!("Awaiting response...");
        let response = res.json::<eflint_json::spec::ResponsePhrases>().await?;

        debug!("Analysing response...");
        let errors: Vec<String> = response
            .results
            .last()
            .map(|r| match r {
                eflint_json::spec::PhraseResult::StateChange(sc) => match &sc.violations {
                    Some(v) => v.iter().map(|v| v.name.clone()).collect(),
                    None => vec![],
                },
                _ => vec![],
            })
            .unwrap_or_else(Vec::new);


        // TODO proper handle invalid query and unexpected result
        let success: Result<bool, String> = response
            .results
            .last()
            .map(|r| match r {
                eflint_json::spec::PhraseResult::BooleanQuery(r) => Ok(r.result),
                eflint_json::spec::PhraseResult::InstanceQuery(_) => Err("Invalid query".into()),
                eflint_json::spec::PhraseResult::StateChange(r) => Ok(r.violated),
            })
            .unwrap_or_else(|| Err("Unexpected result".into()));

        match success {
            Ok(success) => {
                debug!("Response judged as: {}", if success { "success" } else { "violated" });
                Ok(ReasonerResponse::new(success && response.common.success, errors))
            },
            // TODO better error handling
            Err(err) => Err(err.into()),
        }
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
        let question = Phrase::Create(PhraseCreate {
            operand: Expression::ConstructorApp(ExpressionConstructorApp {
                identifier: "task-to-execute".into(),
                operands:   ConstructorInput::ArraySyntax(vec![Expression::Primitive(ExpressionPrimitive::String(task))]),
            }),
        });

        let phrases = self.build_phrases(&policy, state, workflow, question);
        self.process_phrases(&policy, phrases).await
    }

    async fn access_data_request(
        &self,
        policy: Policy,
        state: State,
        workflow: Workflow,
        data: String,
        task: Option<String>,
    ) -> Result<ReasonerResponse, Box<dyn Error>> {
        let question = match task {
            Some(task_id) => {
                info!("Considering data access '{}' for task '{}' in workflow '{}'", data, task_id, workflow.id);
                Phrase::Create(PhraseCreate {
                    operand: Expression::ConstructorApp(ExpressionConstructorApp {
                        identifier: "dataset-to-transfer".into(),
                        operands:   ConstructorInput::ArraySyntax(vec![Expression::Primitive(ExpressionPrimitive::String(data))]),
                    }),
                })
            },
            None => {
                info!("Considering data access '{}' for result of workflow '{}'", data, workflow.id);
                Phrase::Create(PhraseCreate {
                    operand: Expression::ConstructorApp(ExpressionConstructorApp {
                        identifier: "result-to-transfer".into(),
                        operands:   ConstructorInput::ArraySyntax(vec![Expression::Primitive(ExpressionPrimitive::String(data))]),
                    }),
                })
            },
        };

        let phrases = self.build_phrases(&policy, state, workflow, question);
        self.process_phrases(&policy, phrases).await
    }

    async fn workflow_validation_request(&self, policy: Policy, state: State, workflow: Workflow) -> Result<ReasonerResponse, Box<dyn Error>> {
        info!("Considering workflow '{}'", workflow.id);
        let question = Phrase::Create(PhraseCreate {
            operand: Expression::ConstructorApp(ExpressionConstructorApp {
                identifier: "workflow-to-execute".into(),
                operands:   ConstructorInput::ArraySyntax(vec![Expression::Primitive(ExpressionPrimitive::String(workflow.id.clone()))]),
            }),
        });

        let phrases = self.build_phrases(&policy, state, workflow, question);
        self.process_phrases(&policy, phrases).await
    }
}

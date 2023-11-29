use policy::{Policy, PolicyContent, PolicyVersion};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SetVersionPostModel {
    pub version: i64,
}

#[derive(Deserialize)]
pub struct PolicyContentPostModel {
    pub reasoner: String,
    pub reasoner_version: String,
    pub content: Box<serde_json::value::RawValue>,
}

#[derive(Deserialize)]
pub struct AddPolicyPostModel {
    pub description: Option<String>,
    pub version_description: String,
    pub content: Vec<PolicyContentPostModel>,
}

impl AddPolicyPostModel {
    pub fn to_domain(&self) -> Policy {
        Policy {
            description: match self.description.clone() {
                Some(d) => d,
                None => "".into(),
            },
            version:     PolicyVersion {
                creator: None,
                created_at: chrono::Local::now(),
                version: None,
                version_description: self.version_description.clone(),
            },
            content:     self
                .content
                .iter()
                .map(|c| PolicyContent { reasoner: c.reasoner.clone(), reasoner_version: c.reasoner_version.clone(), content: c.content.clone() })
                .collect(),
        }
    }
}

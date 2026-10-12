use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableVariant {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariable {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_type")]
    pub r#type: String, // 'default' | 'secret'
    #[serde(default)]
    pub active_index: usize,
    #[serde(default)]
    pub variants: Vec<VariableVariant>,
}

impl EnvironmentVariable {
    pub fn current_value(&self) -> &str {
        if !self.variants.is_empty() && self.active_index < self.variants.len() {
            &self.variants[self.active_index].value
        } else {
            &self.value
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unresolved {
    pub name: String,
}

fn default_true() -> bool {
    true
}
fn default_type() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub is_active: bool,
    pub variables: Vec<EnvironmentVariable>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub active_environment_id: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

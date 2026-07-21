use std::collections::BTreeMap;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProfileReport {
    pub schema: String,
    pub command: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at_unix_ms: Option<u64>,
    pub total_duration_ns: u64,
    pub spans: Vec<ProfileSpan>,
    pub counters: Vec<ProfileCounter>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProfileSpan {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<u64>,
    pub name: String,
    pub category: String,
    pub start_ns: u64,
    #[serde(default)]
    pub duration_ns: Option<u64>,
    pub status: ProfileSpanStatus,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attrs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProfileCounter {
    pub name: String,
    pub value: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSpanStatus {
    Running,
    Ok,
    Error,
    Abandoned,
}

//! Defines models for requests sent to CUBE.
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FeedSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_exact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_startswith: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_creation_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_creation_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_fname_icontains: Option<String>,
}

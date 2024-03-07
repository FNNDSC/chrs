use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginParameterType {
    Boolean,
    Integer,
    Float,
    String,
    Path,
    Unextpath,
}

/// Plugin instance status
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Created,
    Waiting,
    Scheduled,
    Started,
    RegisteringFiles,
    FinishedSuccessfully,
    FinishedWithError,
    Cancelled,
}

impl Status {
    pub fn simplify(self) -> SimplifiedStatus {
        self.into()
    }
}

/// Simplified variants of [Status].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SimplifiedStatus {
    Waiting,
    Running,
    Success,
    Error,
    Cancelled,
}

impl From<Status> for SimplifiedStatus {
    fn from(value: Status) -> Self {
        match value {
            Status::Created => Self::Waiting,
            Status::Waiting => Self::Waiting,
            Status::Scheduled => Self::Waiting,
            Status::Started => Self::Running,
            Status::RegisteringFiles => Self::Running,
            Status::FinishedSuccessfully => Self::Success,
            Status::FinishedWithError => Self::Error,
            Status::Cancelled => Self::Cancelled,
        }
    }
}

impl PluginParameterType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginParameterType::Boolean => "boolean",
            PluginParameterType::Integer => "int",
            PluginParameterType::Float => "float",
            PluginParameterType::String => "string",
            PluginParameterType::Path => "path",
            PluginParameterType::Unextpath => "unextpath",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum PluginParameterValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),

    /// Either a `str`, `path`, or `unextpath`
    Stringish(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum ParameterValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

/// <https://github.com/FNNDSC/CHRIS_docs/blob/master/specs/ChRIS_Plugins.adoc#plugin-type>
#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Fs,
    Ds,
    Ts,
}

#[derive(Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum PluginParameterAction {
    #[serde(rename = "store")]
    Store,
    #[serde(rename = "store_true")]
    StoreTrue,
    #[serde(rename = "store_false")]
    StoreFalse,
}

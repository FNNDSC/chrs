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

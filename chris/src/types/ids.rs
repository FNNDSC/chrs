use serde::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

/// CUBE User ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct UserId(pub u32);

/// Pipeline ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct PipelineId(pub u32);

/// Plugin ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct PluginId(pub u32);

/// Feed ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct FeedId(pub u32);

/// Feed note ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct NoteId(pub u32);

/// Plugin instance ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct PluginInstanceId(pub u32);

/// Plugin instance parameter ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct PluginInstanceParameterId(pub u32);

/// Plugin parameter ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct PluginParameterId(pub u32);

/// Workflow ID
#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct WorkflowId(pub u32);

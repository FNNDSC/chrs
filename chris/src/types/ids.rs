use serde::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

/// CUBE User ID
#[derive(Shrinkwrap, Deserialize)]
pub struct UserId(pub u32);

/// Pipeline ID
#[derive(Shrinkwrap, Deserialize, Debug)]
pub struct PipelineId(pub u32);

/// Plugin ID
#[derive(Shrinkwrap, Deserialize, Debug)]
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

/// Plugin Parameter ID
#[derive(Shrinkwrap, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub struct PluginParameterId(pub u32);

/// Workflow ID
#[derive(Shrinkwrap, Deserialize, Debug)]
pub struct WorkflowId(pub u32);

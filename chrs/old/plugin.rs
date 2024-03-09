use anyhow::{Context, Ok, Result};
use chris::models::data::{
    ComputeResourceName, PluginInstanceId, PluginName, PluginParameter, PluginParameterAction,
    PluginParameterType, PluginParameterValue,
};
use chris::{ChrisClient, Plugin};
use clap::{Arg, ArgAction, ArgMatches, Command};
use futures::TryStreamExt;
use std::collections::HashMap;

/// Print help for arguments of a plugin.
pub(crate) async fn describe_plugin(chris: &ChrisClient, plugin_name: &PluginName) -> Result<()> {
    let plugin = chris
        .get_plugin_latest(plugin_name)
        .await?
        .with_context(|| format!("plugin not found: {}", plugin_name))?;
    let parameter_info: Vec<PluginParameter> = plugin.get_parameters().try_collect().await?;
    clap_params(&plugin.plugin.selfexec, &parameter_info).print_help()?;
    Ok(())
}

/// Create a plugin instance, given plugin name.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_latest(
    chris: &ChrisClient,
    plugin_name: &PluginName,
    parameters: &[String],
    previous_id: Option<PluginInstanceId>,
    cpu: Option<u16>,
    cpu_limit: Option<String>,
    memory_limit: Option<String>,
    gpu_limit: Option<u32>,
    number_of_workers: Option<u32>,
    compute_resource_name: Option<ComputeResourceName>,
    title: Option<String>,
) -> Result<()> {
    let optional_resources = serialize_optional_resources(
        cpu,
        cpu_limit,
        memory_limit,
        gpu_limit,
        number_of_workers,
        compute_resource_name,
        title,
        previous_id,
    );
    let plugin = chris
        .get_plugin_latest(plugin_name)
        .await?
        .with_context(|| format!("plugin not found: {}", plugin_name))?;

    let mut payload = clap_serialize_params(&plugin, parameters).await?;
    payload.extend(optional_resources);
    let res = plugin.create_instance(&payload).await?;
    println!("{}", res.plugin_instance.url);
    Ok(())
}

fn serialize_optional_resources(
    cpu: Option<u16>,
    cpu_limit: Option<String>,
    memory_limit: Option<String>,
    gpu_limit: Option<u32>,
    number_of_workers: Option<u32>,
    compute_resource_name: Option<ComputeResourceName>,
    title: Option<String>,
    previous_id: Option<PluginInstanceId>,
) -> impl Iterator<Item = (String, PluginParameterValue)> {
    let cpu_limit = cpu.map(|c| format!("{}m", c * 1000)).or(cpu_limit);
    let optional_resources = [
        cpu_limit.map(|v| ("cpu_limit".to_string(), PluginParameterValue::Stringish(v))),
        memory_limit.map(|v| {
            (
                "memory_limit".to_string(),
                PluginParameterValue::Stringish(v),
            )
        }),
        gpu_limit.map(|v| {
            (
                "gpu_limit".to_string(),
                PluginParameterValue::Integer(v as i64),
            )
        }),
        number_of_workers.map(|v| {
            (
                "number_of_workers".to_string(),
                PluginParameterValue::Integer(v as i64),
            )
        }),
        compute_resource_name.map(|v| {
            (
                "compute_resource_name".to_string(),
                PluginParameterValue::Stringish(v.to_string()),
            )
        }),
        title.map(|v| ("title".to_string(), PluginParameterValue::Stringish(v))),
        previous_id.map(|v| {
            (
                "previous_id".to_string(),
                PluginParameterValue::Integer(*v as i64),
            )
        }),
    ];
    optional_resources.into_iter().flatten()
}



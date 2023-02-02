use anyhow::{Context, Ok, Result};
use chris::models::{
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

async fn clap_serialize_params(
    plugin: &Plugin,
    args: &[String],
) -> Result<HashMap<String, PluginParameterValue>> {
    let parameter_info: Vec<PluginParameter> = plugin.get_parameters().try_collect().await?;
    let command = clap_params(&plugin.plugin.selfexec, &parameter_info);
    let matches = command.try_get_matches_from(args)?;

    let parsed_params = parameter_info
        .into_iter()
        .filter_map(|p| get_param_from_matches(p, &matches))
        .collect();

    Ok(parsed_params)
}

fn get_param_from_matches(
    param_info: PluginParameter,
    matches: &ArgMatches,
) -> Option<(String, PluginParameterValue)> {
    // TODO does ChRIS support repeating args?
    let name = param_info.name.as_str();
    let value = match param_info.parameter_type {
        PluginParameterType::Boolean => {
            let value = matches.get_flag(name);
            if (value && param_info.action == PluginParameterAction::StoreTrue)
                || (!value && param_info.action == PluginParameterAction::StoreFalse)
            {
                Some(PluginParameterValue::Boolean(value))
            } else {
                None
            }
        }
        PluginParameterType::Integer => {
            let value: Option<i64> = matches.get_one(name).copied();
            value.map(PluginParameterValue::Integer)
        }
        PluginParameterType::Float => {
            let value: Option<f64> = matches.get_one(name).copied();
            value.map(PluginParameterValue::Float)
        }
        _ => {
            let value: Option<String> = matches.get_one::<String>(name).map(String::from);
            value.map(PluginParameterValue::Stringish)
        }
    };
    value.map(|v| (name.to_string(), v))
}

fn clap_params(selfexec: &str, parameter_info: &[PluginParameter]) -> Command {
    let args = parameter_info.iter().map(pluginparameter2claparg);
    Command::new(selfexec.to_string())
        .no_binary_name(true)
        .disable_help_flag(true)
        .args(args)
}

fn pluginparameter2claparg(param: &PluginParameter) -> Arg {
    let action = match param.action {
        PluginParameterAction::Store => ArgAction::Set,
        PluginParameterAction::StoreTrue => ArgAction::SetTrue,
        PluginParameterAction::StoreFalse => ArgAction::SetFalse,
    };

    let long_flag = get_long_flag_name(param.flag.as_str())
        .get_or_insert(param.name.as_str())
        .to_string();
    let arg = Arg::new(&param.name)
        .value_name(param.parameter_type.as_str())
        .value_parser(clap_parser_for(param.parameter_type))
        .required(!param.optional)
        .help(&param.help)
        .long(long_flag)
        .action(action);

    if let Some(short_flag) = get_short_flag_char(param.short_flag.as_str()) {
        arg.short(short_flag)
    } else {
        arg
    }
}

fn clap_parser_for(t: PluginParameterType) -> clap::builder::ValueParser {
    match t {
        PluginParameterType::Boolean => clap::builder::ValueParser::bool(),
        PluginParameterType::Integer => clap::value_parser!(i64).into(),
        PluginParameterType::Float => clap::value_parser!(f64).into(),
        PluginParameterType::String => clap::builder::ValueParser::string(),
        PluginParameterType::Path => clap::builder::ValueParser::string(),
        PluginParameterType::Unextpath => clap::builder::ValueParser::string(),
    }
}

fn get_short_flag_char(short_flag: &str) -> Option<char> {
    short_flag.split_once('-').and_then(|(lead, name)| {
        if lead.is_empty() {
            let mut chars = name.chars();
            let first = chars.next();
            let second = chars.next();
            if second.is_some() {
                None
            } else {
                first
            }
        } else {
            None
        }
    })
}

fn get_long_flag_name(long_flag: &str) -> Option<&str> {
    long_flag.split_once("--").and_then(|(lead, name)| {
        if lead.is_empty() && !name.is_empty() {
            Some(name)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("-a", Some('a'))]
    #[case("-b", Some('b'))]
    #[case("c", None)]
    #[case("--a", None)]
    #[case("--apple", None)]
    fn test_get_short_flag_char(#[case] short_flag: &str, #[case] expected: Option<char>) {
        assert_eq!(get_short_flag_char(short_flag), expected)
    }

    #[rstest]
    #[case("--apple", Some("apple"))]
    #[case("--ya-pear", Some("ya-pear"))]
    #[case("--ya--pear", Some("ya--pear"))]
    #[case("--y", Some("y"))]
    #[case("ya", None)]
    #[case("-y", None)]
    fn test_get_long_flag_name(#[case] short_flag: &str, #[case] expected: Option<&str>) {
        assert_eq!(get_long_flag_name(short_flag), expected)
    }
}

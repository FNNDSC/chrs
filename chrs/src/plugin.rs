use anyhow::{bail, Context, Ok, Result};
use chris::models::{
    PluginInstanceId, PluginName, PluginParameter, PluginParameterAction, PluginParameterType,
    PluginParameterValue, PluginType,
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
pub(crate) async fn run_latest(
    chris: &ChrisClient,
    plugin_name: &PluginName,
    previous_id: &PluginInstanceId,
    parameters: &[String],
) -> Result<()> {
    let plugin = chris
        .get_plugin_latest(plugin_name)
        .await?
        .with_context(|| format!("plugin not found: {}", plugin_name))?;
    if plugin.plugin.plugin_type == PluginType::Fs {
        bail!("fs plugin type not supported.");
    }

    let mut payload = clap_serialize_params(&plugin, parameters).await?;
    payload.insert(
        "previous_id".to_string(),
        PluginParameterValue::Integer(previous_id.0 as i64),
    );
    let res = plugin.create_instance(&payload).await?;
    println!("{}", res.plugin_instance.url);
    Ok(())
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
        PluginParameterType::String => {
            let value: Option<String> = matches.get_one::<String>(name).map(String::from);
            value.map(PluginParameterValue::Stringish)
        }
        PluginParameterType::Path => {
            let value: Option<String> = matches.get_one::<String>(name).map(String::from);
            value.map(PluginParameterValue::Stringish)
        }
        PluginParameterType::Unextpath => {
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
        .help(&param.help)
        .long(long_flag)
        .action(action);

    if let Some(short_flag) = get_short_flag_char(param.short_flag.as_str()) {
        arg.short(short_flag)
    } else {
        arg
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
            } else if let Some(char) = first {
                Some(char)
            } else {
                None
            }
        } else {
            None
        }
    })
}

fn get_long_flag_name(long_flag: &str) -> Option<&str> {
    long_flag.split_once("--").and_then(|(lead, name)| {
        if lead.is_empty() && name.len() >= 1 {
            Some(name)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chris::models::{PluginParameterAction, PluginParameterId, PluginParameterUrl, PluginUrl};
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

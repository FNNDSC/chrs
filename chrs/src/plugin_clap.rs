//! Helper functions for producing clap commands of ChRIS plugins.

use std::collections::HashMap;

use clap::builder::NonEmptyStringValueParser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use color_eyre::eyre;
use futures::TryStreamExt;

use chris::types::{PluginParameterAction, PluginParameterType, PluginParameterValue};
use chris::{Access, Plugin, PluginParameter};

use crate::arg::GivenDataNode;

/// clap arg ID for plugin input
pub const CHRS_INCOMING: &str = "chrs-incoming-cfb8a325-fbfc-4467-b7d1-4975d1a249cf";

/// Use clap to serialize user-specified `args` for a `plugin`.
pub async fn clap_serialize_params<A: Access>(
    plugin: &Plugin<A>,
    args: &[String],
) -> eyre::Result<(HashMap<String, PluginParameterValue>, Vec<GivenDataNode>)> {
    let parameter_info: Vec<_> = plugin.parameters().stream().try_collect().await?;
    let command = clap_params(&plugin.object.selfexec, &parameter_info);
    parse_args_using(command, &parameter_info, args)
}

pub fn clap_params(selfexec: &str, parameter_info: &[PluginParameter]) -> Command {
    let args = parameter_info.iter().map(pluginparameter2claparg);
    let input_arg = Arg::new(CHRS_INCOMING)
        .help("Plugin instance or feed to use as input for this plugin")
        .value_parser(NonEmptyStringValueParser::new())
        .value_name("incoming")
        .action(ArgAction::Append);
    Command::new(selfexec.to_string())
        .no_binary_name(true)
        .disable_help_flag(true)
        .args(args)
        .arg(input_arg)
}

fn parse_args_using(
    command: Command,
    parameter_info: &[PluginParameter],
    args: &[String],
) -> eyre::Result<(HashMap<String, PluginParameterValue>, Vec<GivenDataNode>)> {
    let matches = command.try_get_matches_from(args)?;
    let parsed_params = parameter_info
        .iter()
        .filter_map(|p| get_param_from_matches(p, &matches))
        .collect();
    let incoming = matches
        .get_many::<String>(CHRS_INCOMING)
        .map(|values| values.map(|s| s.to_string().into()).collect())
        .unwrap_or(Vec::with_capacity(0));
    Ok((parsed_params, incoming))
}

fn get_param_from_matches(
    param_info: &PluginParameter,
    matches: &ArgMatches,
) -> Option<(String, PluginParameterValue)> {
    // Future work: does ChRIS support repeating args?
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
    use rstest::*;

    use chris::types::PluginParameterId;

    use super::*;

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

    const EXAMPLE_PARAMS: &'static [(&str, PluginParameterType, PluginParameterAction, bool)] = &[
        (
            "fun",
            PluginParameterType::Boolean,
            PluginParameterAction::StoreTrue,
            true,
        ),
        (
            "not-boring",
            PluginParameterType::Boolean,
            PluginParameterAction::StoreFalse,
            true,
        ),
        (
            "haoma",
            PluginParameterType::Integer,
            PluginParameterAction::Store,
            true,
        ),
        (
            "score",
            PluginParameterType::Float,
            PluginParameterAction::Store,
            false,
        ),
        (
            "comment",
            PluginParameterType::String,
            PluginParameterAction::Store,
            true,
        ),
    ];

    #[fixture]
    #[once]
    fn params() -> Vec<PluginParameter> {
        EXAMPLE_PARAMS
            .iter()
            .enumerate()
            .map(|(i, (name, parameter_type, action, optional))| {
                let c = name.chars().next().unwrap();
                PluginParameter {
                    url: format!("https://example.com/api/v1/plugins/parameters/{i}").into(),
                    id: PluginParameterId(i as u32),
                    name: name.to_string(),
                    parameter_type: *parameter_type,
                    optional: *optional,
                    default: None,
                    flag: format!("--{name}"),
                    short_flag: format!("-{c}"),
                    action: *action,
                    help: format!("help message for \"{name}\""),
                    ui_exposed: true,
                    plugin: "https://example.com/api/v1/plugins/2/".into(),
                }
            })
            .collect()
    }

    #[fixture]
    fn command(params: &[PluginParameter]) -> Command {
        clap_params("unit test for plugin_clap", params)
    }

    #[rstest]
    fn test_parse_args_not_optional_param(command: Command, params: &[PluginParameter]) {
        let e = parse_args_using(command, params, &["--fun".to_string()])
            .expect_err("--score should be required");
        let msg = e.to_string();
        let expected_msg = "the following required arguments were not provided:";
        let pos = msg
            .find(expected_msg)
            .expect("error message should say \"required arguments were not provided\"");
        let rest_of_msg = &msg[pos + expected_msg.len()..];
        assert!(rest_of_msg.contains("--score <float>"))
    }
}

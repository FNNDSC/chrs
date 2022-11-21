use anyhow::{bail, Context, Ok, Result};
use chris::models::{
    PluginInstanceId, PluginName, PluginParameter, PluginParameterAction, PluginParameterType,
    PluginParameterValue, PluginType,
};
use chris::{ChrisClient, Plugin};
use clap::{Arg, ArgAction, ArgMatches, Command};
use futures::{StreamExt, TryStreamExt};
use std::collections::HashMap;
use itertools::Itertools;
use serde_json::to_string;

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
    // let param_lookup = index_parameters(parameter_info);
    // let mut payload = serialize_params(parameters, &param_lookup)?;
    let mut payload = clap_serialize_params(&plugin, parameters).await?;
    payload.insert("previous_id".to_string(), PluginParameterValue::Integer(previous_id.0 as i64));
    let res = plugin.create_instance(&payload).await?;
    println!("{}", res.plugin_instance.url);
    Ok(())
}

pub(crate) async fn describe_plugin(
    chris: &ChrisClient,
    plugin_name: &PluginName
) -> Result<()> {
    let plugin = chris
        .get_plugin_latest(plugin_name)
        .await?
        .with_context(|| format!("plugin not found: {}", plugin_name))?;
    clap_params(&plugin).await?.print_help()?;
    Ok(())
}

/// Convert cmd-specified arguments to a payload which can be sent via
/// [chris::Plugin::create_instance]
fn serialize_params(
    given: &[String],
    params: &HashMap<String, Box<PluginParameter>>,
) -> Result<Vec<(String, PluginParameterValue)>> {
    if let Some(flag) = given.first() {
        let info = params
            .get(flag)
            .ok_or_else(|| anyhow::Error::msg(format!("Unrecognized parameter: {}", flag)))?;
        let (serialized_param, rest) = serialize_param(flag, given, info)?;
        let mut all_serialized_params = serialize_params(rest, params)?;
        all_serialized_params.push(serialized_param);
        Ok(all_serialized_params)
    } else {
        Ok(vec![])
    }
}

fn serialize_param<'a>(
    flag: &str,
    given: &'a [String],
    info: &'a PluginParameter,
) -> Result<((String, PluginParameterValue), &'a [String])> {
    // TODO handle both --key value and --key=value
    let end = given.len();
    match info.action {
        PluginParameterAction::Store => {
            let raw = given
                .get(1)
                .ok_or_else(|| anyhow::Error::msg(format!("missing value for {}", flag)))?;
            let param = match info.parameter_type {
                PluginParameterType::Integer => raw
                    .parse()
                    .map(PluginParameterValue::Integer)
                    .map_err(anyhow::Error::from),
                PluginParameterType::Float => raw
                    .parse()
                    .map(PluginParameterValue::Float)
                    .map_err(anyhow::Error::from),
                PluginParameterType::String => Ok(PluginParameterValue::Stringish(raw.to_string())),
                PluginParameterType::Path => Ok(PluginParameterValue::Stringish(raw.to_string())),
                PluginParameterType::Unextpath => {
                    Ok(PluginParameterValue::Stringish(raw.to_string()))
                }
                PluginParameterType::Boolean => {
                    let msg = format!(
                        "Bad parameter information from CUBE: {} \
                    has action=\"store\" so type cannot be \"boolean\"",
                        info.url
                    );
                    Err(anyhow::Error::msg(msg))
                }
            }?;
            Ok(((info.name.clone(), param), &given[2..end]))
        }
        PluginParameterAction::StoreTrue => Ok((
            (info.name.clone(), PluginParameterValue::Boolean(true)),
            &given[1..end],
        )),
        PluginParameterAction::StoreFalse => Ok((
            (info.name.clone(), PluginParameterValue::Boolean(false)),
            &given[1..end],
        )),
    }
}

async fn clap_params(plugin: &Plugin) -> Result<Command> {
    let args: Vec<Arg> = plugin
        .get_parameters()
        .map_ok(pluginparameter2claparg)
        .try_collect()
        .await
        .with_context(|| format!("plugin parameters info from \"{}\" is not valid: ", plugin.plugin.parameters))?;
    let command = Command::new(&plugin.plugin.selfexec)
        .no_binary_name(true)
        .disable_help_flag(true)
        .args(args);
    Ok(command)
}

fn pluginparameter2claparg(param: PluginParameter) -> Arg {
    let action = match param.action {
        PluginParameterAction::Store => {ArgAction::Set}
        PluginParameterAction::StoreTrue => {ArgAction::SetTrue}
        PluginParameterAction::StoreFalse => {ArgAction::SetFalse}
    };

    let long_flag = get_long_flag_name(param.flag.as_str()).get_or_insert(param.name.as_str()).to_string();
    let arg = Arg::new(param.name)
        .help(param.help)
        .long(long_flag)
        .action(action);

    if let Some(short_flag) = get_short_flag_char(param.short_flag.as_str()) {
        arg.short(short_flag)
    } else {
        arg
    }
}

async fn clap_serialize_params(plugin: &Plugin, args: &[String]) -> Result<HashMap<String, PluginParameterValue>> {
    let parameter_info: Vec<PluginParameter> = plugin.get_parameters().try_collect().await?;
    let command = clap_params_1(&plugin.plugin.selfexec, &parameter_info);
    let matches = command.try_get_matches_from(args)?;

    let parsed_params = parameter_info
        .into_iter()
        .filter_map(|p| get_param_from_matches(p, &matches))
        .collect();

    Ok(parsed_params)
}

fn get_param_from_matches(param_info: PluginParameter, matches: &ArgMatches) -> Option<(String, PluginParameterValue)> {
    // TODO does ChRIS support repeating args?
    let name = param_info.name.as_str();
    dbg!(&param_info);
    let value = match param_info.parameter_type {
        PluginParameterType::Boolean => {
            let value = matches.get_flag(name);
            if (value && param_info.action == PluginParameterAction::StoreTrue) ||
                (!value && param_info.action == PluginParameterAction::StoreFalse) {
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

fn clap_params_1(selfexec: &str, parameter_info: &[PluginParameter]) -> Command {
    let args = parameter_info.iter().map(pluginparameter2claparg_1);
    Command::new(selfexec.to_string())
        .no_binary_name(true)
        .disable_help_flag(true)
        .args(args)
}

fn pluginparameter2claparg_1(param: &PluginParameter) -> Arg {
    let action = match param.action {
        PluginParameterAction::Store => {ArgAction::Set}
        PluginParameterAction::StoreTrue => {ArgAction::SetTrue}
        PluginParameterAction::StoreFalse => {ArgAction::SetFalse}
    };

    let long_flag = get_long_flag_name(param.flag.as_str()).get_or_insert(param.name.as_str()).to_string();
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
        }
        else {
            None
        }
    })
}

/// Create a mapping of flag -> param, short_flag -> param.
fn index_parameters(
    parameters: impl IntoIterator<Item = PluginParameter>,
) -> HashMap<String, Box<PluginParameter>> {
    let mut map: HashMap<String, Box<PluginParameter>> = parameters
        .into_iter()
        .map(|p| (p.flag.clone(), Box::new(p)))
        .collect();
    let boxed_params: Vec<Box<PluginParameter>> = map.values().cloned().collect();
    for param in boxed_params {
        map.insert(param.short_flag.clone(), param);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use chris::models::{PluginParameterAction, PluginParameterId, PluginParameterUrl, PluginUrl};
    use rstest::*;

    #[fixture]
    fn p_fruit() -> PluginParameter {
        PluginParameter {
            url: PluginParameterUrl::from("https://example.com/api/v1/plugins/parameters/20/"),
            id: PluginParameterId(20),
            name: "fruit".to_string(),
            parameter_type: PluginParameterType::String,
            optional: false,
            default: None,
            flag: "--fruit".to_string(),
            short_flag: "-f".to_string(),
            action: PluginParameterAction::Store,
            help: "name of a common fruit".to_string(),
            ui_exposed: true,
            plugin: PluginUrl::from("https://example.com/api/v1/plugins/3/"),
        }
    }

    #[fixture]
    fn p_veggie() -> PluginParameter {
        PluginParameter {
            url: PluginParameterUrl::from("https://example.com/api/v1/plugins/parameters/21/"),
            id: PluginParameterId(21),
            name: "veggie".to_string(),
            parameter_type: PluginParameterType::Boolean,
            optional: true,
            default: Some(PluginParameterValue::Boolean(false)),
            flag: "--veggie".to_string(),
            short_flag: "--veggie".to_string(),
            action: PluginParameterAction::StoreTrue,
            help: "whether or not is considered a veggie".to_string(),
            ui_exposed: true,
            plugin: PluginUrl::from("https://example.com/api/v1/plugins/3/"),
        }
    }

    #[fixture]
    fn example_lookup(
        p_fruit: PluginParameter,
        p_veggie: PluginParameter,
    ) -> HashMap<String, Box<PluginParameter>> {
        let bp_fruit = Box::new(p_fruit);
        let bp_veggie = Box::new(p_veggie);
        HashMap::from([
            ("-f".to_string(), bp_fruit.clone()),
            ("--fruit".to_string(), bp_fruit.clone()),
            ("--veggie".to_string(), bp_veggie.clone()),
        ])
    }

    #[rstest]
    fn test_index_parameters(
        p_fruit: PluginParameter,
        p_veggie: PluginParameter,
        example_lookup: HashMap<String, Box<PluginParameter>>,
    ) -> Result<()> {
        let given_parameters = vec![p_fruit.clone(), p_veggie.clone()];
        let mut actual = index_parameters(given_parameters);

        for (flag, expected_boxed_param) in example_lookup {
            let actual_boxed_param = actual.remove(&*flag).expect(&*format!(
                "'{}' not found in: {:?}",
                flag,
                actual.keys()
            ));
            assert_eq!(*expected_boxed_param, *actual_boxed_param);
        }
        assert_eq!(actual.len(), 0, "extra flags: {:?}", actual.keys());
        Ok(())
    }

    #[rstest]
    #[case(&["--boba"])]
    #[case(&["--veggie", "--boba"])]
    #[case(&["--boba", "--veggie"])]
    #[case(&["-f", "apple", "--boba"])]
    #[case(&["-f", "apple", "--boba", "--veggie"])]
    fn test_serialize_params_unexpected(
        #[case] given: &[&str],
        example_lookup: HashMap<String, Box<PluginParameter>>,
    ) {
        let given: Vec<String> = given.into_iter().map(|s| s.to_string()).collect();
        assert_eq!(
            serialize_params(&given, &example_lookup)
                .unwrap_err()
                .to_string(),
            "Unrecognized parameter: --boba"
        );
    }

    #[rstest]
    #[case(&[], vec![])]
    #[case(&["--veggie"], vec![("veggie", PluginParameterValue::Boolean(true))])]
    #[case(&["--fruit", "apple"], vec![("fruit", PluginParameterValue::Stringish("apple".to_string()))])]
    #[case(&["-f", "apple"], vec![("fruit", PluginParameterValue::Stringish("apple".to_string()))])]
    #[case(&["-f", "apple", "--veggie"], vec![("veggie", PluginParameterValue::Boolean(true)), ("fruit", PluginParameterValue::Stringish("apple".to_string()))])]
    fn test_serialize_params_works(
        #[case] given: &[&str],
        #[case] expected: Vec<(&str, PluginParameterValue)>,
        example_lookup: HashMap<String, Box<PluginParameter>>,
    ) {
        let given: Vec<String> = given.into_iter().map(|s| s.to_string()).collect();
        let expected: Vec<(String, PluginParameterValue)> = expected
            .into_iter()
            .map(|(s, v)| (s.to_string(), v))
            .collect();

        let actual = serialize_params(&given, &example_lookup).unwrap();
        assert_eq!(expected.len(), actual.len());
        for (e, a) in expected.into_iter().zip(actual.into_iter()) {
            assert_eq!(e, a)
        }
    }

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

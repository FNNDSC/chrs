use anyhow::{bail, Context, Ok, Result};
use chris::models::{
    PluginInstanceId, PluginName, PluginParameter, PluginParameterAction, PluginParameterType,
    PluginParameterValue, PluginType,
};
use chris::ChrisClient;
use futures::TryStreamExt;
use std::collections::HashMap;

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
    let parameter_info: Vec<PluginParameter> = plugin.get_parameters().try_collect().await?;
    let param_lookup = index_parameters(parameter_info);
    let mut payload = serialize_params(parameters, &param_lookup)?;
    payload.push((
        "previous_id".to_string(),
        PluginParameterValue::Integer(previous_id.0 as i64),
    ));
    let body: HashMap<String, PluginParameterValue> = payload.into_iter().collect();
    let res = plugin.create_instance(&body).await?;
    println!("{}", res.plugin_instance.url);
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
            default: PluginParameterValue::Stringish("apple".to_string()),
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
            default: PluginParameterValue::Boolean(false),
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
}

use std::error::Error;
use std::result::Result;
use std::str::FromStr;

/// Prompt for interactive keyboard input if given value is [None].
pub fn prompt_if_missing<T: FromStr>(
    x: Option<T>,
    prompt: &str,
) -> Result<T, ManualInputError<<T as FromStr>::Err>>
where  // janky return type, cannot use anyhow::Error here (really can't use dyn Error)
    <T as FromStr>::Err: Error,

{
    match x {
        Some(v) => Ok(v),
        None => {
            let input: String = dialoguer::Input::new()
                .with_prompt(prompt)
                .interact_text()
                .map_err(ManualInputError::IoError)?;
            Ok(T::from_str(&*input).map_err(ManualInputError::ValueError)?)
        }
    }
}

/// Same as [prompt_if_missing] but input is hidden as it's typed.
pub fn prompt_if_missing_password<T: FromStr>(
    x: Option<T>,
    prompt: &str,
) -> Result<T, ManualInputError<<T as FromStr>::Err>>
where
    <T as FromStr>::Err: Error,
{
    match x {
        Some(v) => Ok(v),
        None => {
            let input: String = dialoguer::Password::new()
                .with_prompt(prompt)
                .interact()
                .map_err(ManualInputError::IoError)?;
            Ok(T::from_str(&*input).map_err(ManualInputError::ValueError)?)
        }
    }
}

/// Errors which may occur while prompting for user input that is to be parsed into a NewType.
#[derive(thiserror::Error, Debug)]
pub enum ManualInputError<E: Error> {
    #[error(transparent)]
    IoError(std::io::Error),
    #[error(transparent)]
    ValueError(E),
}

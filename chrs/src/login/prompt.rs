use std::error::Error;
use std::io;
use std::result::Result;
use std::str::FromStr;

/// Prompt for interactive keyboard input if given value is [None].
pub fn prompt_if_missing<T: FromStr>(
    x: Option<T>,
    prompt: &str,
) -> Result<T, ManualInputError<<T as FromStr>::Err>>
where
    <T as FromStr>::Err: Error,
    // janky return type, cannot use anyhow::Error here (really can't use dyn Error)
{
    match x {
        Some(v) => Ok(v),
        None => {
            let input: String = dialoguer::Input::new()
                .with_prompt(prompt)
                .interact_text()
                .map_err(ManualInputError::Dialoguer)?;
            Ok(T::from_str(&input).map_err(ManualInputError::ValueError)?)
        }
    }
}

/// Same as [prompt_if_missing] but input is hidden as it's typed.
pub fn prompt_if_missing_password<T: FromStr>(
    x: Option<T>,
    prompt: &str,
    from_stdin: bool,
) -> Result<T, ManualInputError<<T as FromStr>::Err>>
where
    <T as FromStr>::Err: Error,
{
    match x {
        Some(v) => Ok(v),
        None => {
            let input: String = if from_stdin {
                read_line().map_err(ManualInputError::IoError)?
            } else {
                dialoguer::Password::new()
                    .with_prompt(prompt)
                    .interact()
                    .map_err(ManualInputError::Dialoguer)?
            };
            Ok(T::from_str(&input).map_err(ManualInputError::ValueError)?)
        }
    }
}

/// Read a line from stdin.
/// <https://doc.rust-lang.org/std/io/struct.Stdin.html#examples>
fn read_line() -> io::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer)
}

/// Errors which may occur while prompting for user input that is to be parsed into a NewType.
#[derive(thiserror::Error, Debug)]
pub enum ManualInputError<E: Error> {
    #[error(transparent)]
    IoError(io::Error),
    #[error(transparent)]
    Dialoguer(dialoguer::Error),
    #[error(transparent)]
    ValueError(E),
}

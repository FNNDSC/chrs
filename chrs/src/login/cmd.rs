use anyhow::{Context, Result, Ok};
use chris::auth::CUBEAuth;
use chris::types::{CUBEApiUrl, Username};


pub async fn login(
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
    password: Option<String>
) -> Result<()> {
    let given_address = address.context("--address is required")?;
    let given_username = username.context("--username is required")?;
    let given_password = password.context("--password is required")?;

    let account = CUBEAuth {
        client: &Default::default(),
        url: &given_address,
        username: &given_username,
        password: given_password.as_str(),
    };

    let token = account.get_token().await?;
    println!("token: {}", token);
    Ok(())
}

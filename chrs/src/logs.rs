use color_eyre::eyre::{OptionExt, Result};

use crate::arg::GivenDataNode;
use crate::credentials::Credentials;

pub async fn logs(credentials: Credentials, given: Option<GivenDataNode>) -> Result<()> {
    let (client, old, _) = credentials
        .get_client(given.as_ref().map(|g| g.as_arg_str()).as_slice())
        .await?;
    let given = given
        .or_else(|| old.map(|id| id.into()))
        .ok_or_eyre("missing operand")?;
    let plinst = given.into_plinst_either(&client, old).await?;
    print!("{}", plinst.logs());
    Ok(())
}

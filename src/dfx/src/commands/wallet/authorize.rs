use crate::commands::wallet::wallet_update;
use crate::lib::environment::Environment;
use crate::lib::error::DfxResult;

use clap::Parser;
use ic_types::Principal;

/// Authorize a wallet custodian.
#[derive(Parser)]
pub struct AuthorizeOpts {
    /// Principal of the custodian to authorize.
    custodian: String,
}

pub async fn exec(env: &dyn Environment, opts: AuthorizeOpts) -> DfxResult {
    let custodian = Principal::from_text(opts.custodian)?;
    wallet_update(env, "authorize", custodian).await?;
    println!("Authorized {} as a custodian.", custodian);
    Ok(())
}

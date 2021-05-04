use crate::commands::ledger::{get_icpts_from_args, send_and_notify};
use crate::lib::environment::Environment;
use crate::lib::error::DfxResult;
use crate::lib::nns_types::account_identifier::Subaccount;
use crate::lib::nns_types::icpts::{ICPTs, TRANSACTION_FEE};
use crate::lib::nns_types::{CyclesResponse, Memo};

use crate::util::clap::validators::{e8s_validator, icpts_amount_validator};

use anyhow::anyhow;
use clap::{ArgSettings, Clap};
use ic_types::principal::Principal;
use std::str::FromStr;

const MEMO_CREATE_CANISTER: u64 = 1095062083_u64;

/// Create a canister from ICP
#[derive(Clap)]
pub struct CreateCanisterOpts {
    /// ICP to mint into cycles and deposit into destination canister
    /// Can be specified as a Decimal with the fractional portion up to 8 decimal places
    /// i.e. 100.012
    #[clap(long, validator(icpts_amount_validator))]
    amount: Option<String>,

    /// Specify ICP as a whole number, helpful for use in conjunction with `--e8s`
    #[clap(long, validator(e8s_validator), conflicts_with("amount"))]
    icp: Option<String>,

    /// Specify e8s as a whole number, helpful for use in conjunction with `--icp`
    #[clap(long, validator(e8s_validator), conflicts_with("amount"))]
    e8s: Option<String>,

    /// Transaction fee, default is 10000 Doms.
    #[clap(long, validator(icpts_amount_validator), setting = ArgSettings::Hidden)]
    fee: Option<String>,

    /// Specify the controller of the new canister
    #[clap(long)]
    controller: String,

    /// Max fee
    #[clap(long, validator(icpts_amount_validator), setting = ArgSettings::Hidden)]
    max_fee: Option<String>,
}

pub async fn exec(env: &dyn Environment, opts: CreateCanisterOpts) -> DfxResult {
    let amount = get_icpts_from_args(opts.amount, opts.icp, opts.e8s)?;

    let fee = opts.fee.map_or(Ok(TRANSACTION_FEE), |v| {
        ICPTs::from_str(&v).map_err(|err| anyhow!(err))
    })?;

    // validated by memo_validator
    let memo = Memo(MEMO_CREATE_CANISTER);

    let to_subaccount = Some(Subaccount::from(&Principal::from_text(opts.controller)?));

    let max_fee = opts
        .max_fee
        .map_or(ICPTs::new(0, 0).map_err(|err| anyhow!(err)), |v| {
            ICPTs::from_str(&v).map_err(|err| anyhow!(err))
        })?;

    let result = send_and_notify(env, memo, amount, fee, to_subaccount, max_fee).await?;

    match result {
        CyclesResponse::CanisterCreated(v) => {
            println!("Canister created with id: {:?}", v.to_text());
        }
        CyclesResponse::Refunded(msg, maybe_block_height) => {
            println!("Refunded with message: {} at {:?}", msg, maybe_block_height);
        }
        CyclesResponse::ToppedUp(()) => unreachable!(),
    };
    Ok(())
}
use crate::commands::build::err_on_command_failure;
use crate::config::dfinity::{ConfigCanistersCanister, Config};
use crate::lib::env::{BinaryResolverEnv, ProjectConfigEnv};
use crate::lib::error::{DfxError, DfxResult};
use crate::lib::message::UserMessage;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::process::Stdio;

const CANISTER_ARG: &str = "canister";

pub fn construct() -> App<'static, 'static> {
    SubCommand::with_name("ide")
        .about(UserMessage::StartIDE.to_str())
        .arg(Arg::with_name(CANISTER_ARG).help(UserMessage::CanisterName.to_str()))
}

pub fn exec<T>(env: &T, args: &ArgMatches<'_>) -> DfxResult
where
    T: BinaryResolverEnv + ProjectConfigEnv,
{
    let config = &env
        .get_config()
        .ok_or(DfxError::CommandMustBeRunInAProject)?;

    let main_path = get_main_path(config, args)?;

    run_ide(env, main_path)
}

fn get_main_path(config: &Config, args: &ArgMatches<'_>) -> Result<String, DfxError> {
    let dfx_json = config.get_path().to_str().unwrap();

    let canister_name: Option<&str> = args.value_of(CANISTER_ARG);

    let (canister_name, canister): (String, ConfigCanistersCanister) =
        match (config.config.canisters.as_ref(), canister_name) {
            (None, _) => Err(DfxError::InvalidData(format!(
                "Missing field defaults.start.serve_root in {0}",
                dfx_json
            ))),

            (Some(canisters), Some(cn)) => {
                let c = canisters.get(cn).ok_or(DfxError::InvalidArgument(format!(
                    "Canister {0} cannot not be found in {1}",
                    cn, dfx_json
                )))?;
                Ok((cn.to_string(), c.clone()))
            }
            (Some(canisters), None) => {
                if canisters.len() == 1 {
                    let (n, c) = canisters.iter().next().unwrap();
                    Ok((n.to_string(), c.clone()))
                } else {
                    Err(DfxError::InvalidData(format!(
                    "There are multiple canisters in {0}, please select one using the {1} argument",
                    dfx_json, CANISTER_ARG
                )))
                }
            }
        }?;

    canister.main.ok_or(DfxError::InvalidData(format!(
        "Canister {0} lacks a 'main' element in {1}",
        canister_name, dfx_json
    )))
}

fn run_ide<T: BinaryResolverEnv>(env: &T, main_path: String) -> DfxResult {
    let output = env
        .get_binary_command("mo-ide")?
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .arg("--canister-main")
        .arg(main_path)
        .output()?;

    err_on_command_failure(output, DfxError::IdeError)
}

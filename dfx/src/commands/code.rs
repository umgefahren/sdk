use std::path::PathBuf;
use crate::commands::build::err_on_command_failure;
use crate::lib::env::{BinaryResolverEnv, ProjectConfigEnv};
use crate::lib::error::{DfxError, DfxResult};
use crate::lib::message::UserMessage;
use clap::{App, ArgMatches, SubCommand};

pub fn construct() -> App<'static, 'static> {
    SubCommand::with_name("code")
        .about(UserMessage::StartCode.to_str())
}

pub fn exec<T>(env: &T, _args: &ArgMatches<'_>) -> DfxResult
where
    T: BinaryResolverEnv + ProjectConfigEnv,
{
    // check this is run in a project
    &env
        .get_config()
        .ok_or(DfxError::CommandMustBeRunInAProject)?;

    let vsix_path = get_vsix_path(env)?;
    run_code(env, &vsix_path)
}

fn run_code<T : BinaryResolverEnv>(env: &T, vsix_path: &PathBuf) -> DfxResult
    where
        T: BinaryResolverEnv + ProjectConfigEnv,
    {
    let vsix_path = vsix_path.as_path();
    let code_err = DfxError::IdeError;

    // install the extension
    let output = env
        .get_binary_command("code")
        .map_err(|_| DfxError::UnknownCommand("Can't find the code executable in your path, do you have VSCode installed?".to_string()))?
        .arg("--install-extension")
        .arg(vsix_path)
        .output()?;

    err_on_command_failure(output, code_err)?;

    let project_root = env
        .get_config()
        .ok_or(DfxError::CommandMustBeRunInAProject)?
        .get_path()
        .parent().unwrap();

    // Run vscode
    let output = env
        .get_binary_command("code")?
        .arg(project_root)
        .output()?;

    err_on_command_failure(output, code_err)
}

fn get_vsix_path<T: BinaryResolverEnv>(env: &T) -> DfxResult<PathBuf> {
    env.get_binary_command_path("vscode-motoko.vsix")
}

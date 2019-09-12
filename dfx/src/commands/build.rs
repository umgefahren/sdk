use crate::config::dfinity::ConfigCanistersCanister;
use crate::lib::build::{build_file, watch_file};
use crate::lib::env::{BinaryResolverEnv, ProjectConfigEnv};
use crate::lib::error::DfxResult;
use clap::{App, Arg, ArgMatches, SubCommand};

pub fn construct() -> App<'static, 'static> {
    SubCommand::with_name("build")
        .about("Build a canister code, or all canisters if no argument is passed.")
        .arg(
            Arg::with_name("canister")
                .help("The canister name to build.")
                .takes_value(true),
        )
        .arg(Arg::with_name("watch").help("Watches the build. By default build and exit."))
}

pub fn exec<T>(env: &T, args: &ArgMatches<'_>) -> DfxResult
where
    T: BinaryResolverEnv + ProjectConfigEnv,
{
    // Read the config.
    let config = env.get_config().unwrap();
    // get_path() returns the full path of the config file. We need to get the dirname.
    let project_root = config.get_path().parent().unwrap();

    let watch_mode = args.occurrences_of("watch") > 0;

    let build_root = project_root.join(
        config
            .get_config()
            .get_defaults()
            .get_build()
            .get_output("build/"),
    );

    if let Some(canisters) = &config.get_config().canisters {
        if watch_mode {
            for (k, v) in canisters {
                let v: ConfigCanistersCanister = serde_json::from_value(v.to_owned())?;

                if let Some(x) = v.main {
                    let input_as_path = project_root.join(x.as_str());
                    let output_path = build_root.join(x.as_str()).with_extension("wasm");
                    std::fs::create_dir_all(output_path.parent().unwrap())?;

                    watch_file(
                        Box::new(env.clone()),
                        &input_as_path,
                        &output_path,
                        Box::new(|p| println!("Rebuilding {}...", p.display())),
                        Box::new(|_| println!("Done")),
                        Box::new(|e| println!("Error: {:?}", e)),
                    )?;
                }
            }
        } else {
            for (k, v) in canisters {
                let v: ConfigCanistersCanister = serde_json::from_value(v.to_owned())?;

                if let Some(x) = v.main {
                    println!("Building {}...", k);
                    let input_as_path = project_root.join(x.as_str());
                    let output_path = build_root.join(x.as_str()).with_extension("wasm");
                    std::fs::create_dir_all(output_path.parent().unwrap())?;

                    build_file(env, &input_as_path, &output_path)?;
                }
            }
        }
    }

    Ok(())
}

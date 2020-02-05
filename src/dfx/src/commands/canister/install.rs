use crate::commands::canister::create_waiter;
use crate::lib::canister_info::CanisterInfo;
use crate::lib::environment::Environment;
use crate::lib::error::{DfxError, DfxResult};
use crate::lib::message::UserMessage;
use clap::{App, Arg, ArgMatches, SubCommand};
use ic_http_agent::{Agent, Blob, RequestId};
use tokio::runtime::Runtime;

pub fn construct() -> App<'static, 'static> {
    SubCommand::with_name("install")
        .about(UserMessage::InstallCanister.to_str())
        .arg(
            Arg::with_name("canister_name")
                .takes_value(true)
                .required_unless("all")
                .help(UserMessage::InstallCanisterName.to_str())
                .required(false),
        )
        .arg(
            Arg::with_name("all")
                .long("all")
                .required_unless("canister_name")
                .help(UserMessage::InstallAll.to_str())
                .takes_value(false),
        )
        .arg(
            Arg::with_name("async")
                .help(UserMessage::AsyncResult.to_str())
                .long("async")
                .takes_value(false),
        )
}

async fn install_canister(agent: &Agent, canister_info: &CanisterInfo) -> DfxResult<RequestId> {
    let canister_id = canister_info.get_canister_id().ok_or_else(|| {
        DfxError::CannotFindBuildOutputForCanister(canister_info.get_name().to_owned())
    })?;

    eprintln!(
        "Installing code for canister {}, with canister_id {}",
        canister_info.get_name(),
        canister_id.to_text(),
    );

    let wasm_path = canister_info.get_output_wasm_path();
    let wasm = std::fs::read(wasm_path)?;

    agent
        .install(&canister_id, &Blob::from(wasm), &Blob::empty())
        .await
        .map_err(DfxError::from)
}

pub fn exec(env: &dyn Environment, args: &ArgMatches<'_>) -> DfxResult {
    let config = env
        .get_config()
        .ok_or(DfxError::CommandMustBeRunInAProject)?;
    let agent = env
        .get_agent()
        .ok_or(DfxError::CommandMustBeRunInAProject)?;
    let mut runtime = Runtime::new().expect("Unable to create a runtime");

    if let Some(canister_name) = args.value_of("canister_name") {
        let canister_info = CanisterInfo::load(&config, canister_name)?;
        let request_id = runtime.block_on(install_canister(&agent, &canister_info))?;

        if args.is_present("async") {
            eprint!("Request ID: ");
            println!("0x{}", String::from(request_id));
            Ok(())
        } else {
            runtime
                .block_on(agent.request_status_and_wait(&request_id, create_waiter()))
                .map(|_| ())
                .map_err(DfxError::from)
        }
    } else if args.is_present("all") {
        // Install all canisters.
        if let Some(canisters) = &config.get_config().canisters {
            for canister_name in canisters.keys() {
                let canister_info = CanisterInfo::load(&config, canister_name)?;
                let request_id = runtime.block_on(install_canister(&agent, &canister_info))?;

                if args.is_present("async") {
                    eprint!("Request ID: ");
                    println!("0x{}", String::from(request_id));
                } else {
                    runtime
                        .block_on(agent.request_status_and_wait(&request_id, create_waiter()))?;
                }
            }
        }
        Ok(())
    } else {
        Err(DfxError::CanisterNameMissing())
    }
}

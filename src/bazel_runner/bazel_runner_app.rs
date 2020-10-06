#[macro_use]
extern crate log;

use clap::{AppSettings, Clap};
use std::{collections::HashMap, path::PathBuf};

use std::env;
use tonic::transport::Server;

use bazelfe::protos::*;
use tokio::prelude::*;

use bazelfe::bazel_runner;
use bazelfe::build_events::error_type_extractor::ErrorInfo;
use bazelfe::error_extraction::scala::stream_operator::ExtractClassData;
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use google::devtools::build::v1::PublishBuildToolEventStreamRequest;
use rand::Rng;
use tokio::sync::broadcast;

#[derive(Clap, Debug)]
#[clap(name = "basic", setting = AppSettings::TrailingVarArg)]
struct Opt {
    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    #[clap(long, env = "INDEX_INPUT_LOCATION", parse(from_os_str))]
    index_input_location: Option<PathBuf>,

    #[clap(required = true, min_values = 1)]
    passthrough_args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();
    let mut rng = rand::thread_rng();

    bazel_runner::register_ctrlc_handler();

    let aes = bazel_runner::action_event_stream::ActionEventStream::new(opt.index_input_location);

    let default_port = {
        let rand_v: u16 = rng.gen();
        40000 + (rand_v % 3000)
    };

    let addr: std::net::SocketAddr = opt
        .bind_address
        .map(|s| s.to_owned())
        .or(env::var("BIND_ADDRESS").ok())
        .unwrap_or_else(|| format!("127.0.0.1:{}", default_port).into())
        .parse()
        .expect("can't parse BIND_ADDRESS variable");

    let passthrough_args = opt.passthrough_args.clone();
    info!("Services listening on {}", addr);

    let (bes, sender_arc, mut rx) =
        bazelfe::build_events::build_event_server::build_bazel_build_events_service();

    let mut error_stream = ErrorInfo::build_transformer(rx);

    let mut target_extracted_stream = aes.build_action_pipeline(error_stream);

    let recv_task = tokio::spawn(async move {
        while let Some(action) = target_extracted_stream.recv().await {
            match action {
                None => println!("Build completed"),
                Some(err_info) => {
                    println!("Error info: {:?}", err_info);
                }
            }
        }
    });

    let bes_port: u16 = addr.port();

    let _service_fut = tokio::spawn(async move {
        Server::builder()
            .add_service(PublishBuildEventServer::new(bes))
            .serve(addr)
            .await
            .unwrap();
        println!("Service is up.");
    });

    let res = bazel_runner::execute_bazel(passthrough_args.clone(), bes_port).await;
    let _ = {
        let mut locked = sender_arc.lock().await;
        locked.take();
    };
    // let _ = bes.write_channel.take();

    println!("{:?}", res);
    println!("Awaiting task...");
    recv_task.await?;
    println!("Task completed...");

    let (tx, rx) = broadcast::channel(256);
    let _ = {
        let mut locked = sender_arc.lock().await;
        *locked = Some(tx);
    };

    let mut error_stream = ErrorInfo::build_transformer(rx);

    let mut target_extracted_stream = aes.build_action_pipeline(error_stream);

    let recv_task = tokio::spawn(async move {
        while let Some(action) = target_extracted_stream.recv().await {
            match action {
                None => println!("Build completed"),
                Some(err_info) => {
                    println!("Error info: {:?}", err_info);
                }
            }
        }
    });
    let res = bazel_runner::execute_bazel(passthrough_args.clone(), bes_port).await;

    let _ = {
        let mut locked = sender_arc.lock().await;
        locked.take();
    };
    // let _ = bes.write_channel.take();

    println!("{:?}", res);
    println!("Awaiting task...");
    recv_task.await?;
    println!("Task completed...");

    Ok(())
}

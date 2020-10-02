#[macro_use]
extern crate log;

use clap::{AppSettings, Clap};

use std::{collections::HashMap, env};
use tonic::transport::Server;

use ::prost::Message;
use bazelfe::build_events::build_event_server::{BuildEventAction, BuildEventService};
use bazelfe::protos::*;
use tokio::prelude::*;

use bazelfe::bazel_runner;
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use google::devtools::build::v1::PublishBuildToolEventStreamRequest;
use rand::Rng;
use tokio::sync::broadcast;

#[derive(Clap, Debug)]
#[clap(name = "basic", setting = AppSettings::TrailingVarArg)]
struct Opt {
    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    #[clap(required = true, min_values = 1)]
    passthrough_args: Vec<String>,
}

fn transform_fn(e: &mut PublishBuildToolEventStreamRequest) -> Option<Vec<u8>> {
    let mut buf = vec![];
    e.encode_length_delimited(&mut buf).unwrap();
    // println!("{:?}", e);
    Some(buf)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();
    let mut rng = rand::thread_rng();

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

    info!("Services listening on {}", addr);

    let (tx, mut rx) = broadcast::channel(32);

    let bes = BuildEventService {
        write_channel: tx,
        transform_fn: std::sync::Arc::new(transform_fn),
    };

    tokio::spawn(async move {
        let mut file: Option<tokio::fs::File> = None;
        let mut idx: u32 = 0;
        while let Ok(action) = rx.recv().await {
            match action {
                BuildEventAction::BuildCompleted => {
                    let _ = file.take();
                    ()
                }
                BuildEventAction::LifecycleEvent(_) => (),
                BuildEventAction::BuildEvent(msg) => {
                    match file {
                        None => {
                            idx = idx + 1;
                            let f = tokio::fs::File::create(format!("build_events_{}.proto", idx))
                                .await
                                .unwrap();
                            file = Some(f);
                        }
                        Some(_) => (),
                    };

                    if let Some(ref mut f) = file {
                        let _res = f.write(&msg).await.unwrap();
                    }
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

    let res = bazel_runner::execute_bazel(opt.passthrough_args, bes_port, HashMap::new()).await;
    println!("{:?}", res);
    // service_fut.await?;

    Ok(())
}

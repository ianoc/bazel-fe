#[macro_use]
extern crate log;

use clap::{AppSettings, Clap};

use std::env;
use tonic::transport::Server;

use ::prost::Message;
use bazelfe::build_events::build_event_server::bazel_event;
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

    let (bes, mut rx) =
        bazelfe::build_events::build_event_server::build_bazel_build_events_service();

    tokio::spawn(async move {
        while let Ok(action) = rx.recv().await {
            match action {
                BuildEventAction::BuildCompleted => (),
                BuildEventAction::LifecycleEvent(_) => (),

                BuildEventAction::BuildEvent(msg) => match msg.event {
                    bazel_event::Evt::BazelEvent(_) => println!("Other message"),
                    bazel_event::Evt::TargetConfigured(a, b) => {
                        println!("Label: {:?}, other: {:?}", a, b)
                    }
                    bazel_event::Evt::ActionCompleted(ace) => println!("Test failure: {:?}", ace),
                    bazel_event::Evt::TestFailure(tfe) => println!("Test failure: {:?}", tfe),
                    bazel_event::Evt::UnknownEvent(String) => (),
                },
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

    let res = bazel_runner::execute_bazel(opt.passthrough_args, bes_port).await;
    println!("{:?}", res);

    Ok(())
}

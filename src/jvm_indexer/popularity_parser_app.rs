use clap::{AppSettings, Clap};
#[macro_use]
extern crate log;
use regex::Regex;

use lazy_static::lazy_static;

use std::time::Instant;
use std::{collections::HashMap, path::PathBuf};

use std::env;
use std::sync::atomic::Ordering;
use tonic::transport::Server;

use bazelfe::protos::*;

use bazelfe::bazel_runner;
use bazelfe::build_events::build_event_server::bazel_event;
use bazelfe::build_events::build_event_server::BuildEventAction;
use bazelfe::build_events::hydrated_stream::HydratedInfo;
use bazelfe::buildozer_driver;
use bazelfe::jvm_indexer::bazel_query::BazelQuery;
use dashmap::{DashMap, DashSet};
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();

    let ret = bazelfe::jvm_indexer::popularity_parser::build_popularity_map();
    for (k, v) in ret {
        println!("{} - {:#?}", v, k);
    }
    Ok(())
}

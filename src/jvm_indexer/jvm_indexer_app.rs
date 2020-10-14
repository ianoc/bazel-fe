#[macro_use]
extern crate log;

use clap::{AppSettings, Clap};
use std::{collections::HashMap, path::PathBuf};

use std::env;
use std::sync::atomic::Ordering;
use tonic::transport::Server;

use bazelfe::protos::*;

use bazelfe::bazel_runner;
use bazelfe::build_events::build_event_server::bazel_event;
use bazelfe::build_events::build_event_server::BuildEventAction;
use bazelfe::build_events::error_type_extractor::ErrorInfo;
use bazelfe::buildozer_driver;
use bazelfe::jvm_indexer::bazel_query::BazelQuery;
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clap, Debug)]
#[clap(name = "basic", setting = AppSettings::TrailingVarArg)]
struct Opt {
    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    #[clap(long, parse(from_os_str))]
    bazel_binary_path: PathBuf,

    #[clap(long, env = "INDEX_OUTPUT_LOCATION", parse(from_os_str))]
    index_output_location: Option<PathBuf>,

    #[clap(long)]
    extra_allowed_rule_kinds: Option<Vec<String>>,
}

fn build_rule_queries(
    extra_allowed_rule_kinds: Option<Vec<String>>,
    target_roots: &Vec<String>,
) -> Vec<String> {
    let allowed_rule_kinds: Vec<String> = vec![
        "java_library",
        "scala_library",
        "scala_proto_library",
        "scala_macro_library",
        "java_proto_library",
        "_java_grpc_library",
    ]
    .into_iter()
    .map(|e| e.to_string())
    .chain(extra_allowed_rule_kinds.unwrap_or_default().into_iter())
    .collect();

    let mut result = Vec::default();
    for target_root in target_roots {
        for allowed_kind in allowed_rule_kinds.iter() {
            result.push(format!("kind({}, {})", allowed_kind, target_root))
        }
    }
    result
}
// async fn spawn_bazel_attempt<T>(
//     sender_arc: &Arc<
//         Mutex<Option<broadcast::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
//     >,
//     aes: &bazel_runner::action_event_stream::ActionEventStream<T>,
//     bes_port: u16,
//     passthrough_args: &Vec<String>,
// ) -> (u32, bazel_runner::ExecuteResult)
// where
//     T: bazelfe::buildozer_driver::Buildozer + Send + Clone + Sync + 'static,
// {
//     let (tx, rx) = broadcast::channel(8192);
//     let _ = {
//         let mut locked = sender_arc.lock().await;
//         *locked = Some(tx);
//     };
//     let error_stream = ErrorInfo::build_transformer(rx);

//     let mut target_extracted_stream = aes.build_action_pipeline(error_stream);

//     let actions_completed: Arc<std::sync::atomic::AtomicU32> =
//         Arc::new(std::sync::atomic::AtomicU32::new(0));

//     let recv_ver = Arc::clone(&actions_completed);
//     let recv_task = tokio::spawn(async move {
//         while let Some(action) = target_extracted_stream.recv().await {
//             match action {
//                 None => (),
//                 Some(err_info) => {
//                     recv_ver.fetch_add(err_info, Ordering::Relaxed);
//                 }
//             }
//         }
//     });
//     let res = bazel_runner::execute_bazel(passthrough_args.clone(), bes_port).await;

//     info!("Bazel completed with state: {:?}", res);
//     let _ = {
//         let mut locked = sender_arc.lock().await;
//         locked.take();
//     };

//     recv_task.await.unwrap();
//     info!("Receive task done");
//     (actions_completed.fetch_add(0, Ordering::Relaxed), res)
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();
    let mut rng = rand::thread_rng();
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.format_timestamp_nanos();
    builder.target(env_logger::fmt::Target::Stderr);
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    } else {
        // builder.parse_filters(String::from("warn,bazelfe::jvm_indexer=info"));
        builder.parse_filters("info");
    }
    builder.init();

    // let aes = bazel_runner::action_event_stream::ActionEventStream::new(
    //     opt.index_input_location,
    //     buildozer_driver::from_binary_path(opt.buildozer_path),
    // );

    // let default_port = {
    //     let rand_v: u16 = rng.gen();
    //     40000 + (rand_v % 3000)
    // };

    // let addr: std::net::SocketAddr = opt
    //     .bind_address
    //     .map(|s| s.to_owned())
    //     .or(env::var("BIND_ADDRESS").ok())
    //     .unwrap_or_else(|| format!("127.0.0.1:{}", default_port).into())
    //     .parse()
    //     .expect("can't parse BIND_ADDRESS variable");

    // let passthrough_args = opt.passthrough_args.clone();
    // info!("Services listening on {}", addr);

    // let (bes, sender_arc, _) =
    //     bazelfe::build_events::build_event_server::build_bazel_build_events_service();

    // let bes_port: u16 = addr.port();

    // let _service_fut = tokio::spawn(async move {
    //     Server::builder()
    //         .add_service(PublishBuildEventServer::new(bes))
    //         .serve(addr)
    //         .await
    //         .unwrap();
    // });

    // let mut attempts: u16 = 0;

    // let mut final_exit_code = 0;
    // while attempts < 15 {
    //     let (actions_corrected, bazel_result) =
    //         spawn_bazel_attempt(&sender_arc, &aes, bes_port, &passthrough_args).await;
    //     final_exit_code = bazel_result.exit_code;
    //     if bazel_result.exit_code == 0 || actions_corrected == 0 {
    //         break;
    //     }
    //     attempts += 1;
    // }

    // println!("Attempts/build cycles: {:?}", attempts);
    // std::process::exit(final_exit_code);

    info!("Executing initial query to find all external repos in this bazel repository");
    let bazel_query = bazelfe::jvm_indexer::bazel_query::from_binary_path(opt.bazel_binary_path);

    let res = bazel_query
        .execute(&vec![
            String::from("query"),
            String::from("--keep_going"),
            String::from("//external:*"),
        ])
        .await;

    let mut target_roots = vec![String::from("//...")];
    for line in res.stdout.lines().into_iter() {
        if let Some(ln) = line.strip_prefix("//external:") {
            if !ln.contains("WORKSPACE") {
                target_roots.push(format!("@{}//...", ln));
            }
        }
    }

    if res.exit_code != 0 {
        info!("The bazel query returned something other than exit code zero, this unfortunately can often happen, so we will continue with the data received. We have identified {} target roots", target_roots.len());
    } else {
        info!("We have identified {} target roots", target_roots.len());
    }

    info!("Extracting targets with an allowed rule kind");

    let all_queries = build_rule_queries(opt.extra_allowed_rule_kinds, &target_roots);

    let union_with_spaces_bytes = " union ".as_bytes();

    let mut all_targets_to_use: HashMap<String, Vec<String>> = HashMap::default();
    let mut processed_count = 0;
    for chunk in all_queries.chunks(2000) {
        let merged = {
            let mut buffer = Vec::default();

            for x in chunk {
                if buffer.is_empty() {
                    buffer.write_all(&x.as_bytes()).unwrap();
                } else {
                    buffer.write_all(&union_with_spaces_bytes).unwrap();
                    buffer.write_all(&x.as_bytes()).unwrap();
                }
            }
            String::from_utf8(buffer).unwrap()
        };
        let res = bazel_query
            .execute(&vec![
                String::from("query"),
                String::from("--keep_going"),
                String::from("--noimplicit_deps"),
                String::from("--output"),
                String::from("label_kind"),
                merged,
            ])
            .await;

        for ln in res.stdout.lines() {
            let entries: Vec<&str> = ln.split_whitespace().collect();
            if entries.len() == 3 {
                let entry = all_targets_to_use
                    .entry(entries[0].to_string())
                    .or_insert(Vec::default());
                entry.push(entries[2].to_string());
            }
            // all_targets_to_use.push(ln.to_string());
        }
        processed_count += chunk.len();
        info!(
            "After {} roots, found {} matching targets",
            processed_count,
            all_targets_to_use.values().fold(0, |acc, e| acc + e.len())
        );
    }

    info!("Found targets");
    for (k, v) in all_targets_to_use.iter() {
        info!("{}\t\t\t{}", k, v.len());
    }

    Ok(())
}
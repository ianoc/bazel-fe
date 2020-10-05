// Not entirely sure one would want to keep these layers/separation long term
// right now this separation in writing this makes it easy to catalog the function
// and ensure its tested right.

// maps over the action stream and provides a new stream of just ErrorInfo outputs
// Unknown if we should consume this as a stream and try action failures immediately
// or wait till the operation is done not to mutate things under bazel?

use clap::{AppSettings, Clap};
use std::collections::HashMap;

use std::env;
use tonic::transport::Server;

use super::build_event_server::bazel_event;
use super::build_event_server::{BuildEventAction, BuildEventService};
use crate::protos::*;
use ::prost::Message;
use tokio::prelude::*;

use tokio::sync::broadcast;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ErrorInfo {
    pub label: String,
    pub output_files: Vec<build_event_stream::file::File>,
    pub target_kind: Option<String>,
}

impl ErrorInfo {
    pub fn build_transformer(
        rx: broadcast::Receiver<BuildEventAction<bazel_event::BazelBuildEvent>>,
    ) -> mpsc::Receiver<Option<ErrorInfo>> {
        let (tx, next_rx) = mpsc::channel(256);

        tokio::spawn(async move {
            let mut rule_kind_lookup = HashMap::new();
            while let Ok(action) = rx.recv().await {
                match action {
                    BuildEventAction::BuildCompleted => {
                        rule_kind_lookup.clear();
                        tx.send(None).await.unwrap();
                    }
                    BuildEventAction::LifecycleEvent(_) => (),
                    BuildEventAction::BuildEvent(msg) => match msg.event {
                        bazel_event::Evt::BazelEvent(_) => (),
                        bazel_event::Evt::TargetConfigured(tgt_cfg) => {
                            println!("targetCfg evt: {:?}", tgt_cfg);
                            rule_kind_lookup.insert(tgt_cfg.label, tgt_cfg.rule_kind);
                        }
                        bazel_event::Evt::ActionCompleted(ace) => {
                            if !ace.success {
                                let err_info = ErrorInfo {
                                    output_files: ace
                                        .stdout
                                        .into_iter()
                                        .chain(ace.stderr.into_iter())
                                        .collect(),
                                    target_kind: rule_kind_lookup
                                        .get(&ace.label)
                                        .map(|e| e.clone()),
                                    label: ace.label,
                                };
                                println!("Action failed error info: {:?}", err_info);
                                tx.send(Some(err_info)).await.unwrap();
                            }
                        }
                        bazel_event::Evt::TestFailure(tfe) => {
                            println!("Test failure: {:?}", tfe);
                            let err_info = ErrorInfo {
                                output_files: tfe.failed_files,
                                target_kind: rule_kind_lookup.get(&tfe.label).map(|e| e.clone()),
                                label: tfe.label,
                            };
                            println!("Error Info: {:?}", err_info);
                            tx.send(Some(err_info)).await.unwrap();
                        }
                        bazel_event::Evt::UnknownEvent(_) => (),
                    },
                }
            }
        });
        next_rx
    }
}

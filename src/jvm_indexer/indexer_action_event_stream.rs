use std::{path::PathBuf, sync::Arc};

use crate::build_events::hydrated_stream;

use super::super::index_table;
use crate::buildozer_driver::Buildozer;
use crate::protos::*;
use dashmap::{DashMap, DashSet};
use std::collections::HashSet;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}
#[derive(Clone, Debug)]
pub struct IndexerActionEventStream {
    index_table: Arc<RwLock<index_table::IndexTable>>,
    allowed_rule_kinds: Arc<HashSet<String>>,
}

impl IndexerActionEventStream {
    pub fn new(allowed_rule_kinds: Vec<String>) -> Self {
        let mut allowed = HashSet::new();
        for e in allowed_rule_kinds.into_iter() {
            allowed.insert(e);
        }
        Self {
            index_table: Arc::new(RwLock::new(index_table::IndexTable::default())),
            allowed_rule_kinds: Arc::new(allowed),
        }
    }

    pub fn build_action_pipeline(
        &self,
        mut rx: mpsc::Receiver<Option<hydrated_stream::HydratedInfo>>,
        results_map: Arc<DashMap<String, Vec<String>>>,
    ) -> mpsc::Receiver<Option<usize>> {
        let (mut tx, next_rx) = mpsc::channel(4096);

        let allowed_rule_kind = Arc::clone(&self.allowed_rule_kinds);
        let self_d: IndexerActionEventStream = self.clone();

        tokio::spawn(async move {
            while let Some(action) = rx.recv().await {
                match action {
                    None => {
                        tx.send(None).await.unwrap();
                    }
                    Some(e) => {
                        let e = e.clone();
                        let allowed_rule_kind = Arc::clone(&allowed_rule_kind);
                        let mut tx = tx.clone();
                        let self_d: IndexerActionEventStream = self_d.clone();
                        let results_map = Arc::clone(&results_map);
                        tokio::spawn(async move {
                            match e {
                                hydrated_stream::HydratedInfo::ActionFailed(
                                    action_failed_error_info,
                                ) => {
                                    // let tbl = Arc::clone(&self_d.index_table);
                                    // let v = tbl.read().await;
                                    // let arc = Arc::clone(&self_d.previous_global_seen);

                                    // arc.entry(action_failed_error_info.label.clone())
                                    //     .or_insert(DashSet::new());
                                    // let prev_data =
                                    //     arc.get(&action_failed_error_info.label).unwrap();

                                    // let actions_completed = super::process_missing_dependency_errors::process_missing_dependency_errors(
                                    //         &prev_data,
                                    //         self_d.buildozer,
                                    //         &action_failed_error_info,
                                    //         v.as_ref().unwrap(),
                                    //     ).await;

                                    // if actions_completed > 0 {
                                    //     tx.send(Some(actions_completed)).await.unwrap();
                                    // }
                                }
                                hydrated_stream::HydratedInfo::ActionSuccess(_) => (),
                                hydrated_stream::HydratedInfo::BazelAbort(_) => {
                                    // aborts can/will occur when we loop through things if stuff depends on an external target
                                    // we don't have configured
                                }
                                hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                                    if let Some(ref target_kind) = tce.target_kind {
                                        if allowed_rule_kind.contains(target_kind) {
                                            let mut found_classes = Vec::default();

                                            for of in tce.output_files.iter() {
                                                if let build_event_stream::file::File::Uri(e) = of {
                                                    if e.starts_with("file://") {
                                                        let u: PathBuf = e
                                                            .strip_prefix("file://")
                                                            .unwrap()
                                                            .into();
                                                        let extracted_zip = crate::zip_parse::extract_classes_from_zip(u);
                                                        for file_name in extracted_zip.into_iter() {
                                                            if let Some(without_suffix) =
                                                                file_name.strip_suffix(".class")
                                                            {
                                                                let e = without_suffix
                                                                    .replace("/", ".")
                                                                    .replace("$", ".");

                                                                found_classes.push(
                                                                    e.strip_suffix(".")
                                                                        .unwrap_or(&e)
                                                                        .to_string(),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            tx.send(Some(found_classes.len())).await.unwrap();
                                            results_map.insert(tce.label, found_classes);
                                        }
                                    }
                                }

                                hydrated_stream::HydratedInfo::Progress(progress_info) => {
                                    // let tbl = Arc::clone(&self_d.previous_global_seen);

                                    // let actions_completed =
                                    //     super::process_build_abort_errors::process_progress(
                                    //         self_d.buildozer,
                                    //         &progress_info,
                                    //         tbl,
                                    //     )
                                    //     .await;

                                    // if actions_completed > 0 {
                                    //     tx.send(Some(actions_completed)).await.unwrap();
                                    // }
                                }
                            }
                        });
                    }
                }
            }
        });
        next_rx
    }
}

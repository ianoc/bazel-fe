use std::{path::PathBuf, sync::Arc};

use crate::build_events::hydrated_stream;

use super::super::index_table;
use crate::buildozer_driver::Buildozer;
use dashmap::{DashMap, DashSet};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}
#[derive(Clone, Debug)]
pub struct IndexerActionEventStream {
    index_output_location: PathBuf,
    index_table: Arc<RwLock<index_table::IndexTable>>,
}

impl IndexerActionEventStream {
    pub fn new(index_output_location: PathBuf) -> Self {
        Self {
            index_output_location: index_output_location,
            index_table: Arc::new(RwLock::new(index_table::IndexTable::default())),
        }
    }

    pub fn build_action_pipeline(
        &self,
        mut rx: mpsc::Receiver<Option<hydrated_stream::HydratedInfo>>,
    ) -> mpsc::Receiver<Option<u32>> {
        let (mut tx, next_rx) = mpsc::channel(4096);

        let self_d: IndexerActionEventStream = self.clone();

        tokio::spawn(async move {
            while let Some(action) = rx.recv().await {
                match action {
                    None => {
                        tx.send(None).await.unwrap();
                    }
                    Some(e) => {
                        let e = e.clone();
                        let mut tx = tx.clone();
                        let self_d: IndexerActionEventStream = self_d.clone();
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

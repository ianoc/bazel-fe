use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use crate::build_events::error_type_extractor;

use super::super::error_extraction;
use super::super::index_table;
use super::super::source_dependencies;
use crate::buildozer_driver::Buildozer;
use crate::protos::*;
use error_extraction::ClassImportRequest;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}
#[derive(Clone, Debug)]
pub struct ActionEventStream<T: Buildozer + Send + Sync + Clone + 'static> {
    index_input_location: Option<PathBuf>,
    index_table: Arc<RwLock<Option<index_table::IndexTable>>>,
    previous_global_seen: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    buildozer: T,
}

fn output_error_paths(err_data: &error_type_extractor::ErrorInfo) -> Vec<std::path::PathBuf> {
    err_data
        .output_files
        .iter()
        .flat_map(|e| match e {
            build_event_stream::file::File::Uri(e) => {
                if (e.starts_with("file://")) {
                    let u: PathBuf = e.strip_prefix("file://").unwrap().into();
                    Some(u)
                } else {
                    println!("Path isn't a file, so skipping...{:?}", e);

                    None
                }
            }
            build_event_stream::file::File::Contents(_) => None,
        })
        .collect()
}

async fn path_to_import_requests(
    error_info: &error_type_extractor::ErrorInfo,
    path_to_use: &PathBuf,
) -> Option<Vec<error_extraction::ClassImportRequest>> {
    let loaded_path = tokio::fs::read_to_string(path_to_use).await.unwrap();
    error_extraction::extract_errors(&error_info.target_kind, &loaded_path)
}

impl<T> ActionEventStream<T>
where
    T: Buildozer + Send + Clone + Sync + 'static,
{
    pub fn new(index_input_location: Option<PathBuf>, buildozer: T) -> Self {
        Self {
            index_input_location: index_input_location,
            index_table: Arc::new(RwLock::new(None)),
            previous_global_seen: Arc::new(Mutex::new(HashMap::new())),
            buildozer: buildozer,
        }
    }

    pub async fn ensure_table_loaded(self) -> () {
        let tbl = Arc::clone(&self.index_table);
        let mut v = tbl.read().await;
        if (*v).is_none() {
            drop(v);
            let mut w = tbl.write().await;
            match *w {
                None => {
                    let index_tbl = match &self.index_input_location {
                        Some(p) => {
                            let content = std::fs::read_to_string(p).unwrap();
                            index_table::parse_file(&content).unwrap()
                        }
                        None => index_table::IndexTable::new(),
                    };
                    *w = Some(index_tbl);
                }
                Some(_) => (),
            }
            drop(w);
            v = tbl.read().await;
        }

        ()
    }

    pub fn build_action_pipeline(
        &self,
        mut rx: mpsc::Receiver<Option<error_type_extractor::ErrorInfo>>,
    ) -> mpsc::Receiver<Option<u32>> {
        let (mut tx, next_rx) = mpsc::channel(4096);

        let self_d: ActionEventStream<T> = self.clone();

        tokio::spawn(async move {
            let mut done_load = false;
            while let Some(action) = rx.recv().await {
                match action {
                    None => {
                        tx.send(None).await;
                    }
                    Some(mut e) => {
                        if !done_load {
                            let nxt = self_d.clone();
                            nxt.ensure_table_loaded().await;
                        }
                        e.label = super::sanitization_tools::sanitize_label(e.label);
                        let paths = output_error_paths(&e);

                        for path in paths {
                            let e = e.clone();
                            let path_to_use = path.clone();
                            let mut tx = tx.clone();
                            let self_d: ActionEventStream<T> = self_d.clone();

                            tokio::spawn(async move {
                                let extracted =
                                    path_to_import_requests(&e, &path_to_use.into()).await;

                                debug!("Extracted: {:?}", extracted);
                                let tbl = Arc::clone(&self_d.index_table);
                                let v = tbl.read().await;

                                let actions_completed = match extracted {
                                    None => 0,
                                    Some(ct) => {
                                        let prev_data = {
                                            let arc = Arc::clone(&self_d.previous_global_seen);
                                            let mut locked_item = arc.lock().await;
                                            locked_item.remove(&e.label).unwrap_or(HashSet::new())
                                        };
                                        let (updated_hashset, actions_completed) = super::process_missing_dependency_errors::process_missing_dependency_errors(
                                            ct,
                                            prev_data,
                                            self_d.buildozer,
                                            &e,
                                            v.as_ref().unwrap(),
                                        ).await;

                                        let _ = {
                                            let arc = Arc::clone(&self_d.previous_global_seen);
                                            let mut locked_item = arc.lock().await;
                                            locked_item.insert(e.label.clone(), updated_hashset);
                                        };
                                        actions_completed
                                    }
                                };
                                tx.send(Some(actions_completed)).await;
                            });
                        }
                    }
                }
            }
        });
        next_rx
    }
}

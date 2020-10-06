use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::build_events::error_type_extractor;

use super::super::error_extraction;
use super::super::index_table;
use super::super::source_dependencies;
use crate::protos::*;
use error_extraction::ClassImportRequest;
use tokio::sync::RwLock;

use tokio::sync::mpsc;

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}
#[derive(Clone, Debug)]
pub struct ActionEventStream {
    index_input_location: Option<PathBuf>,
    index_table: Arc<RwLock<Option<index_table::IndexTable>>>,
}

impl ActionEventStream {
    pub fn new(index_input_location: Option<PathBuf>) -> Self {
        Self {
            index_input_location: index_input_location,
            index_table: Arc::new(RwLock::new(None)),
        }
    }
    async fn path_to_import_requests(
        &self,
        path_to_use: &PathBuf,
    ) -> Option<Vec<error_extraction::ClassImportRequest>> {
        println!("{:?}", path_to_use);

        let loaded_path = tokio::fs::read_to_string(path_to_use).await.unwrap();

        println!("{:?}", loaded_path);
        let error_data = error_extraction::extract_errors(path_to_use, &loaded_path);

        let src_data = source_dependencies::parse_path(path_to_use, &loaded_path).unwrap();
        unimplemented!()
    }

    pub async fn lookup_package<S>(&self, key: S) -> Option<Vec<(u16, String)>>
    where
        S: Into<String>,
    {
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

        match v.as_ref() {
            None => None,
            Some(index_table) => index_table.get(key).map(|e| e.clone()),
        }
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

    pub fn build_action_pipeline(
        &self,
        mut rx: mpsc::Receiver<Option<error_type_extractor::ErrorInfo>>,
    ) -> mpsc::Receiver<Option<u32>> {
        let (mut tx, next_rx) = mpsc::channel(256);

        let self_d = self.clone();
        tokio::spawn(async move {
            while let Some(action) = rx.recv().await {
                match action {
                    None => {
                        tx.send(None).await;
                    }
                    Some(mut e) => {
                        e.label = super::sanitization_tools::sanitize_label(e.label);
                        let paths = ActionEventStream::output_error_paths(&e);

                        for path in paths {
                            let path_to_use = path.clone();
                            let mut tx = tx.clone();
                            let self_d = self_d.clone();
                            tokio::spawn(async move {
                                let loaded_path =
                                    tokio::fs::read_to_string(path_to_use).await.unwrap();

                                let extracted =
                                    self_d.path_to_import_requests(&loaded_path.into()).await;
                                tx.send(Some(33)).await;
                            });
                        }
                    }
                }
            }
        });
        next_rx
    }
}

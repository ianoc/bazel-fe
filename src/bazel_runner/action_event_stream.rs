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

    // If you use macros, say the scala_library suite or similar
    // to generate many rule invocations from one call site, you need to collapse these back
    // for us to be able to add deps/action things.
    fn sanitize_label(label: String) -> String {
        match label.find("_auto_gen_") {
            None => label,
            Some(idx) => label[0..idx].to_string(),
        }
    }

    fn prepare_class_import_requests(
        mut class_import_requests: Vec<ClassImportRequest>,
    ) -> Vec<ClassImportRequest> {
        // if a more specific reference to a class/package exists which covers the same package space
        // and that one is allowed recursive search. Then remove the less specific ones, since we will fall back to those
        // via the more specific anyway.

        // First we identify which targets are allowed recursive search.
        let mut recursive_enabled = HashSet::new();
        for e in class_import_requests.iter() {
            if !e.exact_only {
                recursive_enabled.insert(e.class_name.clone());
            }
        }

        // next we prune the existing import requests
        let mut i = 0;
        while i != class_import_requests.len() {
            let element = &class_import_requests[i];
            let mut found = false;
            for recur in recursive_enabled.iter() {
                if recur.contains(&element.class_name) && (*recur) != element.class_name {
                    found = true;
                    break;
                }
            }

            if found {
                let val = class_import_requests.remove(i);
            // your code here
            } else {
                i += 1;
            }
        }
        class_import_requests
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
                    Some(e) => {
                        let paths = ActionEventStream::output_error_paths(&e);
                        let id_info = e.label;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_label() {
        assert_eq!(
            ActionEventStream::sanitize_label(String::from("foo_bar")),
            String::from("foo_bar")
        );

        assert_eq!(
            ActionEventStream::sanitize_label(String::from("foo/bar/baz:werwe_auto_gen_werewr")),
            String::from("foo/bar/baz:werwe")
        );
    }

    #[test]
    fn test_prepare_class_import_requests() {
        let input = vec![
            ClassImportRequest {
                class_name: String::from("asdf.sadf.sdfwer.sdf"),
                exact_only: false,
                src_fn: "unused",
                priority: 1,
            },
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdfee"),
                exact_only: false,
                src_fn: "unused",
                priority: 1,
            },
        ];

        //pass through, no change
        assert_eq!(
            ActionEventStream::prepare_class_import_requests(input),
            vec![
                ClassImportRequest {
                    class_name: String::from("asdf.sadf.sdfwer.sdf"),
                    exact_only: false,
                    src_fn: "unused",
                    priority: 1
                },
                ClassImportRequest {
                    class_name: String::from("foo_bar_baz.sadf.sdfwer.sdfee"),
                    exact_only: false,
                    src_fn: "unused",
                    priority: 1,
                }
            ]
        );

        // subset prune
        let input = vec![
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdf"),
                exact_only: false,
                src_fn: "unused",
                priority: 1,
            },
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf"),
                exact_only: false,
                src_fn: "unused",
                priority: 1,
            },
        ];

        // only the longer one is kept
        assert_eq!(
            ActionEventStream::prepare_class_import_requests(input),
            vec![ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdf"),
                exact_only: false,
                src_fn: "unused",
                priority: 1
            },]
        );

        // cannot prune since set to exact only
        let input = vec![
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdf"),
                exact_only: true,
                src_fn: "unused",
                priority: 1,
            },
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf"),
                exact_only: false,
                src_fn: "unused",
                priority: 1,
            },
        ];

        // only the longer one is kept
        assert_eq!(
            ActionEventStream::prepare_class_import_requests(input),
            vec![
                ClassImportRequest {
                    class_name: String::from("foo_bar_baz.sadf.sdfwer.sdf"),
                    exact_only: true,
                    src_fn: "unused",
                    priority: 1,
                },
                ClassImportRequest {
                    class_name: String::from("foo_bar_baz.sadf"),
                    exact_only: false,
                    src_fn: "unused",
                    priority: 1,
                },
            ]
        );
    }
}

// Not entirely sure one would want to keep these layers/separation long term
// right now this separation in writing this makes it easy to catalog the function
// and ensure its tested right.

// maps over the action stream and provides a new stream of just ErrorInfo outputs
// Unknown if we should consume this as a stream and try action failures immediately
// or wait till the operation is done not to mutate things under bazel?

use std::{collections::HashMap, path::PathBuf};

use super::super::ClassImportRequest;

use tokio::sync::broadcast;
use tokio::sync::mpsc;

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}

// pub fn extract_errors(input: &str) -> Option<Vec<ClassImportRequest>> {
pub fn build_transformer<
    K: Send + 'static,
    U: Send + Clone + 'static,
    T: ExtractClassData<U> + Send + 'static + From<K>,
>(
    mut rx: mpsc::Receiver<Option<K>>,
) -> mpsc::Receiver<Option<(U, Option<Vec<ClassImportRequest>>)>> {
    let (mut tx, next_rx) = mpsc::channel(256);

    tokio::spawn(async move {
        while let Some(action) = rx.recv().await {
            match action {
                None => {
                    tx.send(None).await;
                }
                Some(orig) => {
                    let e: T = orig.into();
                    let paths = e.paths();
                    let id_info = e.id_info();
                    for path in paths {
                        let path_to_use = path.clone();
                        let id_info = id_info.clone();
                        let mut tx = tx.clone();
                        tokio::spawn(async move {
                            let loaded_path = tokio::fs::read_to_string(path_to_use).await.unwrap();

                            let extracted = super::extract_errors(&loaded_path);
                            tx.send(Some((id_info, extracted))).await;
                        });
                    }
                }
            }
        }
    });
    next_rx
}

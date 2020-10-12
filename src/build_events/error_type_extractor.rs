// Not entirely sure one would want to keep these layers/separation long term
// right now this separation in writing this makes it easy to catalog the function
// and ensure its tested right.

// maps over the action stream and provides a new stream of just ErrorInfo outputs
// Unknown if we should consume this as a stream and try action failures immediately
// or wait till the operation is done not to mutate things under bazel?

use std::collections::HashMap;

use super::build_event_server::bazel_event;
use super::build_event_server::BuildEventAction;
use crate::protos::*;

use tokio::sync::broadcast;
use tokio::sync::mpsc;

#[derive(Clone, PartialEq, Debug)]
pub struct ActionFailedErrorInfo {
    pub label: String,
    pub output_files: Vec<build_event_stream::file::File>,
    pub target_kind: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct BazelAbortErrorInfo {
    pub label: Option<String>,
    pub reason: Option<build_event_stream::aborted::AbortReason>,
    pub description: String,
}

//May or may not contain error info not reflected elsewhere?

// Broad strokes of the failure occured inside an action (most common)
// or at a bazel abort, things like mis-configured build files
#[derive(Clone, PartialEq, Debug)]
pub enum ErrorInfo {
    BazelAbort(BazelAbortErrorInfo),
    ActionFailed(ActionFailedErrorInfo),
    Progress(bazel_event::ProgressEvt),
}

impl ErrorInfo {
    pub fn build_transformer(
        mut rx: broadcast::Receiver<BuildEventAction<bazel_event::BazelBuildEvent>>,
    ) -> mpsc::Receiver<Option<ErrorInfo>> {
        let (mut tx, next_rx) = mpsc::channel(256);

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
                            rule_kind_lookup.insert(tgt_cfg.label, tgt_cfg.rule_kind);
                        }
                        bazel_event::Evt::ActionCompleted(ace) => {
                            if !ace.success {
                                let err_info = ActionFailedErrorInfo {
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
                                tx.send(Some(ErrorInfo::ActionFailed(err_info)))
                                    .await
                                    .unwrap();
                            }
                        }

                        bazel_event::Evt::TestFailure(tfe) => {
                            let err_info = ActionFailedErrorInfo {
                                output_files: tfe.failed_files,
                                target_kind: rule_kind_lookup.get(&tfe.label).map(|e| e.clone()),
                                label: tfe.label,
                            };
                            tx.send(Some(ErrorInfo::ActionFailed(err_info)))
                                .await
                                .unwrap();
                        }
                        bazel_event::Evt::Progress(progress) => {
                            tx.send(Some(ErrorInfo::Progress(progress))).await.unwrap();
                        }
                        bazel_event::Evt::Aborted(tfe) => {
                            let err_info = BazelAbortErrorInfo {
                                reason: tfe.reason,
                                description: tfe.description,
                                label: tfe.label,
                            };
                            tx.send(Some(ErrorInfo::BazelAbort(err_info)))
                                .await
                                .unwrap();
                        }
                        bazel_event::Evt::UnknownEvent(_) => (),
                    },
                }
            }
        });
        next_rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_no_history() {
        let (tx, rx) = broadcast::channel(128);
        let mut child_rx = ErrorInfo::build_transformer(rx);

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: None,
                stderr: None,
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(ErrorInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: None,
                label: String::from("foo_bar_baz"),
                output_files: vec![]
            }))
        );
    }

    #[tokio::test]
    async fn test_with_files() {
        let (tx, rx) = broadcast::channel(128);
        let mut child_rx = ErrorInfo::build_transformer(rx);

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stdout",
                ))),
                stderr: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stderr",
                ))),
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(ErrorInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: None,
                label: String::from("foo_bar_baz"),
                output_files: vec![
                    build_event_stream::file::File::Uri(String::from("path-to-stdout",)),
                    build_event_stream::file::File::Uri(String::from("path-to-stderr",))
                ]
            }))
        );
    }

    #[tokio::test]
    async fn test_with_history() {
        let (tx, rx) = broadcast::channel(128);
        let mut child_rx = ErrorInfo::build_transformer(rx);

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::TargetConfigured(bazel_event::TargetConfiguredEvt {
                label: String::from("foo_bar_baz"),
                rule_kind: String::from("my_madeup_rule"),
            }),
        }))
        .unwrap();

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stdout",
                ))),
                stderr: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stderr",
                ))),
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(ErrorInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: Some(String::from("my_madeup_rule")),
                label: String::from("foo_bar_baz"),
                output_files: vec![
                    build_event_stream::file::File::Uri(String::from("path-to-stdout",)),
                    build_event_stream::file::File::Uri(String::from("path-to-stderr",))
                ]
            }))
        );
    }

    #[tokio::test]
    async fn state_resets_on_new_build() {
        let (tx, rx) = broadcast::channel(128);
        let mut child_rx = ErrorInfo::build_transformer(rx);

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::TargetConfigured(bazel_event::TargetConfiguredEvt {
                label: String::from("foo_bar_baz"),
                rule_kind: String::from("my_madeup_rule"),
            }),
        }))
        .unwrap();

        tx.send(BuildEventAction::BuildCompleted).unwrap();

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stdout",
                ))),
                stderr: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stderr",
                ))),
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        // First event is a None to indicate the build is completed.
        assert_eq!(received_res, None);

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(ErrorInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: None,
                label: String::from("foo_bar_baz"),
                output_files: vec![
                    build_event_stream::file::File::Uri(String::from("path-to-stdout",)),
                    build_event_stream::file::File::Uri(String::from("path-to-stderr",))
                ]
            }))
        );
    }
}

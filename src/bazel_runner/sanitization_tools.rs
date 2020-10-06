use std::collections::HashSet;

use crate::build_events::error_type_extractor;

use super::super::error_extraction;
use error_extraction::ClassImportRequest;
pub(in crate::bazel_runner) fn sanitize_label(label: String) -> String {
    // If you use macros, say the scala_library suite or similar
    // to generate many rule invocations from one call site, you need to collapse these back
    // for us to be able to add deps/action things.

    let label = match label.find("_auto_gen_") {
        None => label,
        Some(idx) => label[0..idx].to_string(),
    };

    // Here we are normalizing
    // src/foo/bar/baz and src/foo/bar/baz:baz
    // ensures we don't try refer to ourselves

    let label = match label.find(":") {
        None => {
            let last_segment = &label[label.rfind("/").map(|e| e + 1).unwrap_or(0)..label.len()];
            format!("{}:{}", label, last_segment).to_string()
        }
        Some(_) => label,
    };

    label
}

pub(in crate::bazel_runner) fn prepare_class_import_requests(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_label() {
        assert_eq!(
            sanitize_label(String::from("foo_bar")),
            String::from("foo_bar:foo_bar")
        );

        assert_eq!(
            sanitize_label(String::from("foo/bar/baz:werwe_auto_gen_werewr")),
            String::from("foo/bar/baz:werwe")
        );

        assert_eq!(
            sanitize_label(String::from("foo/bar/baz:foop")),
            String::from("foo/bar/baz:foop")
        );

        assert_eq!(
            sanitize_label(String::from("foo/bar/baz")),
            String::from("foo/bar/baz:baz")
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
            prepare_class_import_requests(input),
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
            prepare_class_import_requests(input),
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
            prepare_class_import_requests(input),
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

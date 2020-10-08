use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use lazy_static::lazy_static;

use crate::{
    build_events::error_type_extractor::ErrorInfo, buildozer_driver::Buildozer, error_extraction,
    index_table,
};

use log;

fn get_candidates_for_class_name(
    error_info: &ErrorInfo,
    class_name: &str,
    index_table: &index_table::IndexTable,
) -> Vec<(u16, String)> {
    lazy_static! {
      // These are things that are already implicit dependencencies so we should ensure they are not included
        static ref FORBIDDEN_TARGETS_BY_TYPE: HashMap<String, HashSet<String>> = {
            let mut m = HashMap::new();
            let mut cur_s = HashSet::new();
            cur_s.insert(String::from(
                "@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library",
            ));
            m.insert(String::from("scala_library"), cur_s);

            let mut cur_s = HashSet::new();
            cur_s.insert(String::from("@third_party_jvm//3rdparty/jvm/org/scalatest"));
            cur_s.insert(String::from(
                "@third_party_jvm//3rdparty/jvm/org/scalatest:scalatest",
            ));
            cur_s.insert(String::from(
                "@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library",
            ));
            m.insert(String::from("scala_test"), cur_s);
            m
        };
    }

    let mut results = index_table
        .get(class_name)
        .map(|e| e.clone())
        .unwrap_or(vec![]);

    match &error_info.target_kind {
        Some(target_kind) => match FORBIDDEN_TARGETS_BY_TYPE.get(target_kind) {
            None => (),
            Some(forbidden_targets) => {
                results = results
                    .into_iter()
                    .filter(|(freq, target)| !forbidden_targets.contains(target))
                    .collect();
            }
        },
        None => (),
    };

    results = results
        .into_iter()
        .chain(super::expand_target_to_guesses::get_guesses_for_class_name(class_name).into_iter())
        .map(|(a, b)| (a, super::sanitization_tools::sanitize_label(b)))
        .collect();

    results.sort_by(|a, b| b.0.cmp(&a.0));
    results
}

pub fn is_potentially_valid_target(label: &str) -> bool {
    let prepared_path = label.strip_prefix("//").and_then(|e| e.split(":").next());
    match prepared_path {
        Some(p) => {
            let path = Path::new(p);
            path.join("BUILD").exists()
        }
        None => true,
    }
}

pub async fn process_missing_dependency_errors<T: Buildozer + Clone + Send + Sync + 'static>(
    candidate_import_requests: Vec<error_extraction::ClassImportRequest>,
    global_previous_seen: HashSet<String>,
    buildozer: T,
    error_info: &ErrorInfo,
    index_table: &index_table::IndexTable,
) -> (HashSet<String>, u32) {
    let mut local_previous_seen: HashSet<String> = HashSet::new();

    let mut candidate_import_requests =
        super::sanitization_tools::expand_candidate_import_requests(candidate_import_requests);

    let mut ignore_dep_referneces: HashSet<String> = {
        let mut to_ignore = HashSet::new();
        let d = buildozer.print_deps(&error_info.label).await.unwrap();
        d.into_iter().for_each(|dep| {
            to_ignore.insert(super::sanitization_tools::sanitize_label(dep));
        });

        global_previous_seen.into_iter().for_each(|dep| {
            to_ignore.insert(super::sanitization_tools::sanitize_label(dep));
        });

        to_ignore.insert(super::sanitization_tools::sanitize_label(
            error_info.label.clone(),
        ));
        to_ignore
    };

    let mut actions_completed: u32 = 0;
    log::debug!("ignore_dep_references: {:?}", ignore_dep_referneces);
    for (_, inner_versions) in candidate_import_requests.into_iter() {
        'class_entry_loop: for class_name in inner_versions {
            let candidates: Vec<(u16, String)> =
                get_candidates_for_class_name(&error_info, &class_name, &index_table);
            log::debug!("Candidates: {:?}", candidates);
            for (_, target_name) in candidates {
                if !ignore_dep_referneces.contains(&target_name)
                    && is_potentially_valid_target(&target_name)
                {
                    // If our top candidate hits to be a local previous seen stop
                    // processing this class
                    if (local_previous_seen.contains(&target_name)) {
                        break 'class_entry_loop;
                    }

                    // otherwise... add the dependency with buildozer here
                    // then add it ot the local seen dependencies
                    log::info!("Calling buildozer for: {:?}", target_name);
                    buildozer
                        .add_dependency(&error_info.label, &target_name)
                        .await
                        .unwrap();
                    actions_completed += 1;

                    local_previous_seen.insert(target_name.clone());

                    // Now that we have a version with a match we can jump right out to the outside
                    break 'class_entry_loop;
                }
            }
        }
    }

    // concat the global perm ignore with the local_previous seen data
    // this becomes our next global ignore for this target
    ignore_dep_referneces.extend(local_previous_seen);
    (ignore_dep_referneces, actions_completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_candidates_from_map() {
        let mut tbl_map = HashMap::new();
        tbl_map.insert(
            String::from("com.example.foo.bar.Baz"),
            vec![(13, String::from("//src/main/foop/blah:oop"))],
        );
        let index_table = index_table::IndexTable::from_hashmap(tbl_map);

        let error_info = ErrorInfo {
            label: String::from("//src/main/foo/asd/we:wer"),
            output_files: vec![],
            target_kind: Some(String::from("scala_library")),
        };

        assert_eq!(
            get_candidates_for_class_name(&error_info, "com.example.bar.Baz", &index_table),
            vec![]
        );

        assert_eq!(
            get_candidates_for_class_name(&error_info, "com.example.foo.bar.Baz", &index_table),
            vec![
                (13, String::from("//src/main/foop/blah:oop")),
                (0, String::from("//src/main/scala/com/example/foo/bar:bar")),
                (0, String::from("//src/main/java/com/example/foo/bar:bar"))
            ]
        );

        assert_eq!(
            get_candidates_for_class_name(&error_info, "com.example.a.b.c.Baz", &index_table),
            vec![
                (0, String::from("//src/main/scala/com/example/a/b/c:c")),
                (0, String::from("//src/main/java/com/example/a/b/c:c"))
            ]
        );
    }
}

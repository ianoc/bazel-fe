use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use lazy_static::lazy_static;

use crate::{
    build_events::error_type_extractor::ErrorInfo, buildozer_driver::Buildozer, error_extraction,
    index_table,
};

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

pub async fn process_missing_dependency_errors<T: Buildozer + Send + 'static>(
    candidate_import_requests: Vec<error_extraction::ClassImportRequest>,
    global_previous_seen: HashSet<String>,
    buildozer: T,
    error_info: &ErrorInfo,
    index_table: &index_table::IndexTable,
    next_failing_target: &ErrorInfo,
    error_ln: &String,
    line_number: u32,
    file_lines: &Vec<String>,
) -> HashSet<String> {
    let mut local_previous_seen: HashSet<String> = HashSet::new();

    let mut candidate_import_requests =
        super::sanitization_tools::expand_candidate_import_requests(candidate_import_requests);

    let ignore_dep_referneces: HashSet<String> = {
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

    for (candidate, inner_versions) in candidate_import_requests.into_iter() {
        'class_entry_loop: for class_name in inner_versions {
            let candidates: Vec<(u16, String)> =
                get_candidates_for_class_name(&error_info, &class_name, &index_table);
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
                    unimplemented!()
                }
            }
        }
    }

    unimplemented!()

    //   candidateClasses
    //   // sort by most specific to least
    //     .foldLeft(localPreviousSeen) {
    //       case (localPreviousSeen, targetInfo) =>
    //         val ClassImportRequest(targetClass, exactMatch, dbgSource, _) = targetInfo

    //         @annotation.tailrec
    //         def go(remainingPieces: List[String], outerUsedUpSoFar: Set[String]): Set[String] = {
    //           val clazzName = remainingPieces.reverse.mkString(".")
    //           logger.debug(
    //             s"[$dbgSource][exactMatch: ${exactMatch}] For class ${targetClass}: looking up - ${clazzName}"
    //           )
    //           val forbiddenTargetsForType =
    //             forbidddenTargetsByType.get(nextFailingTarget.targetKind).getOrElse(Set())

    //           val targetsToTryAdd: List[String] = (classTargetIndex
    //             .get(clazzName)
    //             .map(_.toList)
    //             .getOrElse(Nil)
    //             .filterNot { e =>
    //               forbiddenTargetsForType.contains(e)
    //             } ++ ExpandToTargetGuesses(clazzName))
    //             .filter { t =>
    //               !globalPreviousSeen.contains(t)
    //             }
    //             .filter { candidate =>
    //               // Here we are normalizing
    //               // src/foo/bar/baz and src/foo/bar/baz:baz
    //               // ensures we don't try refer to ourselves
    //               val candidateT =
    //                 if (candidate.contains(":")) candidate
    //                 else s"${candidate}:${candidate.split('/').last}"
    //               val updatedT =
    //                 if (targetLabel.contains(":")) targetLabel
    //                 else s"${targetLabel}:${targetLabel.split('/').last}"
    //               updatedT != candidateT
    //             }
    //             .distinct

    //           targetsToTryAdd.foreach { t =>
    //             logger.info(s"Target to try add: $t for clazz : $clazzName")
    //           }
    //           if (targetsToTryAdd.exists { e =>
    //                 localPreviousSeen.contains(e)
    //               }) {
    //             logger.debug(
    //               s"[$dbgSource] For class ${targetClass}: already applied ${targetsToTryAdd.filter { e =>
    //                 localPreviousSeen.contains(e)
    //               }.headOption}"
    //             )
    //             outerUsedUpSoFar
    //           } else {

    //             @annotation.tailrec
    //             def applyToFirstSuccessfulTarget(
    //                 targetsToTryAdd: List[String],
    //                 usedUpSoFar: Set[String]
    //             ): (Boolean, Set[String]) =
    //               targetsToTryAdd match {
    //                 case nxt :: t if sanitizedCompare(nxt, nextFailingTarget.label) =>
    //                   applyToFirstSuccessfulTarget(t, usedUpSoFar)
    //                 case nxt :: t =>
    //                   val nextUsedUp = usedUpSoFar + nxt
    //                   val buildozerCommand = s"${buildozerPath.s} 'add deps $nxt' ${targetLabel}"
    //                   val retCode = ProcessRunner.executeCommand(Parser.tokenize(buildozerCommand))(
    //                     StreamHandler.blackHole
    //                   )
    //                   if (retCode == 0) {
    //                     logger.debugAction(s"${ln}:$dbgSource")
    //                     System.err.println(
    //                       s"${Console.GREEN}${Console.BOLD}bazel-cmd-helper${Console.GREEN}: Add dependency $nxt to ${targetLabel}. While attempting to repair: ${clazzName} ${Console.RESET}"
    //                     )
    //                     logger.info(
    //                       s"[$dbgSource] [buildozer] added $nxt to ${targetLabel} - searching for class ${clazzName} --- ${buildozerCommand}"
    //                     )
    //                     (true, nextUsedUp)
    //                   } else {
    //                     logger.info(s"[$dbgSource] Buildozer command failed: ${buildozerCommand}")
    //                     applyToFirstSuccessfulTarget(t, nextUsedUp)
    //                   }
    //                 case Nil => (false, Set())
    //               }

    //             applyToFirstSuccessfulTarget(targetsToTryAdd, Set()) match {
    //               case (true, s) => outerUsedUpSoFar ++ s
    //               // Only recurse if we have more than tld.toplevel to recurse on.
    //               // a `com.google` or `com.twitter` is just too generic
    //               case (false, s) if (remainingPieces.size > 3 && !exactMatch) =>
    //                 go(remainingPieces.tail, outerUsedUpSoFar ++ s)
    //               case (false, s) => outerUsedUpSoFar ++ s
    //             }
    //           }
    //         }
    //         globalPreviousSeen ++ localPreviousSeen ++ go(
    //           targetClass.split('.').toList.reverse,
    //           Set()
    //         )
    //     }
    // }
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

use lazy_static::lazy_static;
use regex::Regex;

use super::ClassImportRequest;

// Example usage:
// SCALA:
// package com.example
// import foo.bar.baz

fn build_class_import_request(class_name: String) -> ClassImportRequest {
    ClassImportRequest {
        class_name: class_name,
        exact_only: false,
        src_fn: "extract_object_not_found",
        priority: 1,
    }
}

pub fn extract(input: &str) -> Option<Vec<ClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^src/[^.]*.scala.*error: not found: object (.*)$").unwrap();
    }

    let mut result = None;
    for ln in input.lines() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let class_name = captures.get(1).unwrap().as_str();
                let class_import_request = build_class_import_request(class_name.to_string());
                result = match result {
                    None => Some(vec![class_import_request]),
                    Some(ref mut inner) => {
                        inner.push(class_import_request);
                        result
                    }
                };
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_object_not_found_error() {
        let sample_output =
            "src/main/scala/com/example/Example.scala:40: error: not found: object foo
import foo.bar.baz
       ^
src/main/scala/com/example/Example.scala:40: warning: Unused import
import foo.bar.baz
               ^
one warning found
one error found
one warning found
one error found
java.lang.RuntimeException: Build failed
    at io.bazel.rulesscala.scalac.ScalacProcessor.compileScalaSources(ScalacProcessor.java:244)
    at io.bazel.rulesscala.scalac.ScalacProcessor.processRequest(ScalacProcessor.java:69)
    at io.bazel.rulesscala.worker.GenericWorker.runPersistentWorker(GenericWorker.java:45)
    at io.bazel.rulesscala.worker.GenericWorker.run(GenericWorker.java:111)
    at io.bazel.rulesscala.scalac.ScalaCInvoker.main(ScalaCInvoker.java:41)";

        assert_eq!(
            extract(sample_output),
            Some(vec![build_class_import_request("foo".to_string())])
        );
    }
}

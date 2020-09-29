use lazy_static::lazy_static;
use regex::Regex;

use super::ClassImportRequest;

// Example usage:
// SCALA:
// package com.example
// class Foo extends asdf

fn build_class_import_request(class_name: String) -> ClassImportRequest {
    ClassImportRequest {
        class_name: class_name,
        exact_only: false,
        src_fn: "extract_value_not_found",
        priority: 1,
    }
}

pub fn extract_value_not_found(input: &str) -> Option<Vec<ClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^src/[^.]*.scala.*error: not found: value (.*)$").unwrap();
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
    fn test_value_not_found_error() {
        let sample_output =
            "src/main/scala/com/example/Example.scala:8:: error: not found: value Foop
val myVal = Foop
            ^
one error found
one error found";

        assert_eq!(
            extract_value_not_found(sample_output),
            Some(vec![build_class_import_request("Foop".to_string())])
        );
    }
}

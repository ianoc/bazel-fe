use lazy_static::lazy_static;
use regex::Regex;

use super::JavaClassImportRequest;

// Example usage:
// JAVA:
// package com.example;
// import com.example.foo.bar.Baz;

fn build_class_import_request(
    source_file_name: String,
    class_name: String,
) -> JavaClassImportRequest {
    JavaClassImportRequest {
        src_file_name: source_file_name,
        class_name: class_name,
        exact_only: false,
        src_fn: "package_does_not_exist",
        priority: 1,
    }
}

pub fn extract(input: &str) -> Option<Vec<JavaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^(.*\.java).*error: package ([A-Za-z0-9.<>_]+).* does not exist\s*$")
                .unwrap();
    }

    let mut result = None;
    for ln in input.lines() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();
                let package = captures.get(2).unwrap().as_str();
                let class_import_request =
                    build_class_import_request(src_file_name.to_string(), package.to_string());
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
    fn test_not_a_member_of_package_error() {
        let sample_output =
            "src/main/java/com/example/Example.java:3: error: package com.google.common.base does not exist
    import com.google.common.base.Preconditions;
";
        assert_eq!(
            extract(sample_output),
            Some(vec![build_class_import_request(
                String::from("src/main/java/com/example/Example.java"),
                "com.google.common.base".to_string()
            )])
        );
    }
}

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

pub(in crate::error_extraction) fn extract(
    input: &str,
    file_parse_cache: &mut super::FileParseCache,
) -> Option<Vec<JavaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^(.*\.java):(\d+):.*error: cannot find symbol\s*$").unwrap();
    }

    let mut result = None;
    for ln in input.lines() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();
                let src_line_number: u32 = captures.get(2).unwrap().as_str().parse().unwrap();

                if let Some(file_data) = file_parse_cache.load_file(src_file_name) {
                    for e in file_data.imports.iter() {
                        if e.line_number == src_line_number {
                            let class_import_request = build_class_import_request(
                                src_file_name.to_string(),
                                e.prefix_section.to_string(),
                            );
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
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/foo/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![crate::source_dependencies::Import {
                    line_number: 16,
                    prefix_section: String::from("javax.annotation.Nullable"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );
        let sample_output =
            "src/main/java/com/example/foo/Example.java:16: error: cannot find symbol
import javax.annotation.Nullable;
                       ^
  symbol:   class Nullable
  location: package javax.annotation";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/main/java/com/example/foo/Example.java"),
                "javax.annotation.Nullable".to_string()
            )])
        );
    }
}

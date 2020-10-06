use lazy_static::lazy_static;
use regex::Regex;

use super::super::ClassImportRequest;

// Example usage:
// This is with one plus deps enabled in rules scala
// each target depends on the target before it in the alphabet directly.
// SCALA:
//
// A.scala / target name "A"
//package com.example.a
//   trait ATrait {
//    type Foo
//     def fooA: String
//   }
//
// B.scala / target name "B"
// package com.example.b
//
// C.scala / target name "C"
// package com.example.c
// object Cobject extends com.example.a.ATrait {
//     type Foo = String
//     val value = "asdf"
//     def fooA = ???
//   }

fn build_class_import_request(class_name: String) -> ClassImportRequest {
    ClassImportRequest {
        class_name: class_name,
        exact_only: false,
        src_fn: "extract_symbol_type_missing_from_classpath",
        priority: 1,
    }
}

pub fn extract(input: &str) -> Option<Vec<ClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^src/[^.]*.scala.*error: Symbol 'type ([A-Za-z0-9.<>_]+)' is missing from the classpath.$").unwrap();
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
    fn test_symbol_type_missing_from_classpath_error() {
        let sample_output ="
src/main/scala/com/example/D.scala:9: error: Symbol 'type com.example.a.ATrait' is missing from the classpath.
This symbol is required by 'trait com.example.b.BTraitExtendsA'.
Make sure that type ATrait is in your classpath and check for conflicting dependencies with `-Ylog-classpath`.
A full rebuild may help if 'BTraitExtendsA.class' was compiled against an incompatible version of com.example.a.
object DClass extends com.example.c.CTraitExtendsBTraitExtendsATrait
                                    ^
one error found
one error found";

        assert_eq!(
            extract(sample_output),
            Some(vec![build_class_import_request(
                "com.example.a.ATrait".to_string()
            )])
        );
    }
}

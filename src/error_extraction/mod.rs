use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct ClassImportRequest {
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: String,
    pub priority: u32,
}

pub mod scala;

pub fn extract_errors(
    target_kind: &Option<String>,
    input: &str,
) -> Option<Vec<ClassImportRequest>> {
    match target_kind.as_ref() {
        None => None,
        Some(kind) => match kind.as_ref() {
            "scala_library" => scala::extract_errors(input),
            "scala_test" => scala::extract_errors(input),
            _ => None,
        },
    }
}

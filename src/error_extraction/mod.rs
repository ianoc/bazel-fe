use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct ClassImportRequest {
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: &'static str,
    pub priority: u32,
}

pub mod scala;

pub fn extract_errors(path: &PathBuf, input: &str) -> Option<Vec<ClassImportRequest>> {
    match path.extension() {
        None => None,
        Some(ext) => match ext.to_str() {
            Some("scala") => scala::extract_errors(input),
            _ => None,
        },
    }
}

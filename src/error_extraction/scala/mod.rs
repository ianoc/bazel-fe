#[derive(Debug, PartialEq)]
pub struct ClassImportRequest {
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: &'static str,
    pub priority: u32,
}

mod object_not_found;

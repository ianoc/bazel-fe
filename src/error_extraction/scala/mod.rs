#[derive(Debug, PartialEq)]
pub struct ClassImportRequest {
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: &'static str,
    pub priority: u32,
}

mod error_is_not_a_member_of_package;
mod error_object_not_found;
mod error_symbol_is_missing_from_classpath;
mod error_symbol_type_missing_from_classpath;
mod error_type_not_found;
mod error_value_not_found;

mod error_is_not_a_member_of_package;
mod error_object_not_found;
mod error_symbol_is_missing_from_classpath;
mod error_symbol_type_missing_from_classpath;
mod error_type_not_found;
mod error_value_not_found;

pub fn extract_errors(input: &str) -> Option<Vec<super::ClassImportRequest>> {
    let combined_vec: Vec<super::ClassImportRequest> = vec![
        error_is_not_a_member_of_package::extract(input),
        error_object_not_found::extract(input),
        error_symbol_is_missing_from_classpath::extract(input),
        error_symbol_type_missing_from_classpath::extract(input),
        error_type_not_found::extract(input),
        error_value_not_found::extract(input),
    ]
    .into_iter()
    .flat_map(|e| e.into_iter().flat_map(|inner| inner.into_iter()))
    .collect();

    if combined_vec.is_empty() {
        None
    } else {
        Some(combined_vec)
    }
}

pub mod stream_operator;

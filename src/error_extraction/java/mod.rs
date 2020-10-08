use std::{collections::HashMap, path::Path};

use crate::source_dependencies::ParsedFile;

mod error_cannot_find_symbol;
mod error_indirect_dependency;
mod error_package_does_not_exist;

#[derive(Debug, PartialEq, Clone)]
pub struct JavaClassImportRequest {
    pub src_file_name: String,
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: &'static str,
    pub priority: u32,
}

impl JavaClassImportRequest {
    pub fn to_class_import_request(self) -> super::ClassImportRequest {
        super::ClassImportRequest {
            class_name: self.class_name,
            exact_only: self.exact_only,
            src_fn: format!("java::{}", self.src_fn),
            priority: self.priority,
        }
    }
}

fn do_load_file(path_str: &str) -> Option<ParsedFile> {
    let path = Path::new(path_str);

    if path.exists() {
        let file_contents = std::fs::read_to_string(path).unwrap();
        match crate::source_dependencies::java::parse_file(&file_contents) {
            Err(_) => None,
            Ok(file) => Some(file),
        }
    } else {
        None
    }
}
pub(in crate::error_extraction) struct FileParseCache {
    file_parse_cache: HashMap<String, ParsedFile>,
}
impl FileParseCache {
    pub fn new() -> Self {
        Self {
            file_parse_cache: HashMap::new(),
        }
    }
    pub fn init_from_par(key: String, v: ParsedFile) -> Self {
        let mut map = HashMap::new();
        map.insert(key, v);
        Self {
            file_parse_cache: map,
        }
    }
    pub fn load_file(&mut self, file_path: &str) -> Option<&ParsedFile> {
        if (!self.file_parse_cache.contains_key(file_path)) {
            if let Some(parsed_file) = do_load_file(file_path) {
                self.file_parse_cache
                    .insert(file_path.to_string(), parsed_file);
            }
        }
        self.file_parse_cache.get(file_path)
    }
}
pub fn extract_errors(input: &str) -> Option<Vec<super::ClassImportRequest>> {
    let mut file_parse_cache: FileParseCache = FileParseCache::new();
    let combined_vec: Vec<super::ClassImportRequest> = vec![
        error_package_does_not_exist::extract(input),
        error_indirect_dependency::extract(input),
        error_cannot_find_symbol::extract(input, &mut file_parse_cache),
    ]
    .into_iter()
    .flat_map(|e| e.into_iter().flat_map(|inner| inner.into_iter()))
    .flat_map(|mut e| {
        let cached_file_data = file_parse_cache.load_file(&e.src_file_name);

        match cached_file_data {
            None => vec![e],
            Some(file_data) => {
                let extra_wildcard_imports: Vec<JavaClassImportRequest> = file_data
                    .imports
                    .iter()
                    .filter_map(|e| match e.suffix {
                        crate::source_dependencies::SelectorType::SelectorList(_) => None,
                        crate::source_dependencies::SelectorType::WildcardSelector() => {
                            Some(&e.prefix_section)
                        }
                        crate::source_dependencies::SelectorType::NoSelector => None,
                    })
                    .map(|prefix| JavaClassImportRequest {
                        class_name: format!("{}.{}", prefix, e.class_name),
                        ..e.clone()
                    })
                    .collect();

                extra_wildcard_imports
                    .into_iter()
                    .chain(vec![e].into_iter())
                    .collect()
            }
        }
        .into_iter()
        .map(|o| o.to_class_import_request())
    })
    .collect();

    if combined_vec.is_empty() {
        None
    } else {
        Some(combined_vec)
    }
}

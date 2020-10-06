fn get_guesses_for_class_name(class_name: &String) -> Vec<String> {
    let mut sections: Vec<&str> = class_name.split(".").collect();

    // heuristic looking for a class name, to ignore separate from the package...

    let mut idx = 0;
    let mut found = false;
    while idx < sections.len() {
        let ele = &sections[idx];
        if ele.starts_with(|ch: char| ch.is_uppercase()) {
            found = true;
            break;
        }
        idx += 1;
    }

    if (found) {
        sections.truncate(idx);
    }

    if sections.len() <= 3 {
        return vec![];
    }

    let suffix = sections.join("/");

    vec![
        format!("//src/main/scala/{}", suffix).to_string(),
        format!("//src/main/java/{}", suffix).to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_for_class_name() {
        assert_eq!(
            get_guesses_for_class_name(&String::from("com.example.foo.bar.baz")),
            vec![
                String::from("//src/main/scala/com/example/foo/bar/baz"),
                String::from("//src/main/java/com/example/foo/bar/baz")
            ]
        );
    }

    #[test]
    fn test_guess_for_class_name_too_short() {
        assert_eq!(
            get_guesses_for_class_name(&String::from("com.example.foo")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_guess_for_class_name_strip_class_name() {
        assert_eq!(
            get_guesses_for_class_name(&String::from(
                "com.example.foo.bar.baz.MyObject.InnerObject"
            )),
            vec![
                String::from("//src/main/scala/com/example/foo/bar/baz"),
                String::from("//src/main/java/com/example/foo/bar/baz")
            ]
        );
    }

    #[test]
    fn test_guess_for_class_name_too_short_post_strip() {
        assert_eq!(
            get_guesses_for_class_name(&String::from("com.example.MyObject.MyObject.InnerObject")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_guess_for_class_start_with_class_name() {
        assert_eq!(
            get_guesses_for_class_name(&String::from("MyObject.MyObject.InnerObject")),
            Vec::<String>::new()
        );
    }
}

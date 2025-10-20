pub struct StringUtils;

impl StringUtils {
    pub fn to_pascal_case_with_dots(name: &str) -> String {
        if !name.contains(".") {
            return name.to_owned();
        }

        Self::to_pascal_case(name)
    }

    pub fn to_pascal_case(name: &str) -> String {
        name.split(['_', '-'])
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>()
    }
}

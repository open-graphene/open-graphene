pub trait IntoStringList {
    fn into_string_list(self) -> Vec<String>;
}

impl<const N: usize> IntoStringList for [&str; N] {
    fn into_string_list(self) -> Vec<String> {
        self.into_iter().map(str::to_string).collect()
    }
}

impl IntoStringList for Vec<String> {
    fn into_string_list(self) -> Vec<String> {
        self
    }
}

impl IntoStringList for Vec<&str> {
    fn into_string_list(self) -> Vec<String> {
        self.into_iter().map(str::to_string).collect()
    }
}

impl IntoStringList for Vec<&String> {
    fn into_string_list(self) -> Vec<String> {
        self.into_iter().map(ToString::to_string).collect()
    }
}

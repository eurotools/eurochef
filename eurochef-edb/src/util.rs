pub(crate) fn get_last_path_part(s: &str) -> Option<&str> {
    s.split("::").last()
}

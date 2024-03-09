/// Wrapper for [shlex::try_quote] which never fails. NUL characters are replaced.
pub(crate) fn shlex_quote(in_str: &str) -> String {
    shlex::try_quote(&in_str.replace('\0', "¡NUL!"))
        .unwrap()
        .to_string()
}

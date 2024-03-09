use crate::arg::GivenRunnable;

impl clap::builder::ValueParserFactory for GivenRunnable {
    type Parser = GivenRunnableParser;
    fn value_parser() -> Self::Parser {
        GivenRunnableParser
    }
}

#[derive(Clone, Debug)]
pub struct GivenRunnableParser;
impl clap::builder::TypedValueParser for GivenRunnableParser {
    type Value = GivenRunnable;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        if let Ok(value) = value.to_os_string().into_string() {
            GivenRunnable::try_from(value)
                .map_err(|_| clap::Error::new(clap::error::ErrorKind::InvalidValue))
        } else {
            Err(clap::Error::new(clap::error::ErrorKind::InvalidUtf8))
        }
    }
}

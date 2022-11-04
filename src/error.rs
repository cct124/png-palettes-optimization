use std::fmt;
pub use Error::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Error {
    /// Congratulations, you've discovered an edge case
    Unsupported,
    /// 不支持的png颜色模式
    UnsupportedColorMode,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            Self::Unsupported => "UNSUPPORTED",
            Self::UnsupportedColorMode => "Unsupported_Color_Mode",
        })
    }
}

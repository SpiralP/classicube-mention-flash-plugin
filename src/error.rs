use error_chain::error_chain;
pub use error_chain::{bail, ensure};

error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
        ParseFloatError(::std::num::ParseFloatError);
        ParseIntError(::std::num::ParseIntError);
        ParseBoolError(::std::str::ParseBoolError);
        Utf8Error(::std::str::Utf8Error);
        NulError(::std::ffi::NulError);
        Regex(regex::Error);
    }
}

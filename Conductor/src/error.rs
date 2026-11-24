use std::fmt;

pub const EXIT_OK: i32 = 0;
/// Generic environment/setup failure not covered by the protocol table.
pub const EXIT_INTERNAL: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_CORE_UNAVAILABLE: i32 = 3;
pub const EXIT_TRANSPILE_FAILED: i32 = 4;
pub const EXIT_BUILD_FAILED: i32 = 5;
pub const EXIT_RUN_FAILED: i32 = 6;

/// An error that already carries the process exit code it should produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: i32,
    pub message: String,
}

impl Error {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(EXIT_USAGE, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(EXIT_CORE_UNAVAILABLE, message)
    }

    pub fn transpile(message: impl Into<String>) -> Self {
        Self::new(EXIT_TRANSPILE_FAILED, message)
    }

    pub fn build(message: impl Into<String>) -> Self {
        Self::new(EXIT_BUILD_FAILED, message)
    }

    pub fn run(message: impl Into<String>) -> Self {
        Self::new(EXIT_RUN_FAILED, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(EXIT_INTERNAL, message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_keep_exit_codes() {
        assert_eq!(Error::usage("x").code, EXIT_USAGE);
        assert_eq!(Error::unavailable("x").code, EXIT_CORE_UNAVAILABLE);
        assert_eq!(Error::transpile("x").code, EXIT_TRANSPILE_FAILED);
        assert_eq!(Error::build("x").code, EXIT_BUILD_FAILED);
        assert_eq!(Error::run("x").code, EXIT_RUN_FAILED);
        assert_eq!(Error::internal("x").code, EXIT_INTERNAL);
    }
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected token '{actual}', expected '{expected}'")]
    UnexpectedToken {
        expected: &'static str,
        actual: String,
    },
}

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum SwitchrError {
    ParseError(String),
    BadConfig(String),
}

impl Display for SwitchrError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for SwitchrError {}
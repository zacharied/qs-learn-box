use quicksilver;

use std::{fmt, result};

#[derive(Debug)]
pub enum Error {
    ObstacleRixelOutOfBounds(f32),
    QuicksilverError(quicksilver::Error),
}

pub type Result<T> = result::Result<T, Error>;

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ObstacleRixelOutOfBounds(pos) => {
                write!(f, "Obstacle position {} is out of bonds", pos)
            }
            Error::QuicksilverError(err) => err.fmt(f),
        }
    }
}

impl From<quicksilver::Error> for Error {
    fn from(e: quicksilver::Error) -> Self {
        Error::QuicksilverError(e)
    }
}


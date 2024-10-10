use std::fmt;

#[derive(Debug)]
pub enum MMRError {
    StartGreaterThanEnd,
    InvalidNumberOfPeaks,
    MergeError,
}

impl fmt::Display for MMRError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MMRError::StartGreaterThanEnd => write!(f, "Start index is greater than end index"),
            MMRError::InvalidNumberOfPeaks => {
                write!(f, "Invalid number of peaks for the given range")
            }
            MMRError::MergeError => write!(f, "Error while merging MMRs"),
        }
    }
}

impl std::error::Error for MMRError {}

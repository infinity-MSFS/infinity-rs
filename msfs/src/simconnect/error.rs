//! Error types for SimConnect.

use crate::sys;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by SimConnect operations.
///
/// `Hresult` wraps a non-success HRESULT returned synchronously by the API
/// (for example a closed handle or invalid parameter). `Exception` is produced
/// asynchronously by the simulator and surfaces through the dispatch queue;
/// see [`crate::simconnect::Recv::Exception`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// SimConnect returned a non-success HRESULT.
    Hresult(i32),
    /// `SimConnect_Open` failed (handle is null or HRESULT is non-success).
    OpenFailed(i32),
    /// String passed to SimConnect contained an interior NUL byte.
    InteriorNul,
    /// Tried to operate on a closed/invalid handle.
    Closed,
}

impl Error {
    #[inline]
    pub(crate) fn from_hresult(hr: sys::HRESULT) -> Result<()> {
        if hr >= 0 {
            Ok(())
        } else {
            Err(Error::Hresult(hr as i32))
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hresult(hr) => write!(f, "SimConnect HRESULT 0x{:08X}", *hr as u32),
            Error::OpenFailed(hr) => {
                write!(f, "SimConnect_Open failed with HRESULT 0x{:08X}", *hr as u32)
            }
            Error::InteriorNul => write!(f, "string contained an interior NUL byte"),
            Error::Closed => write!(f, "SimConnect handle has been closed"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::ffi::NulError> for Error {
    fn from(_: std::ffi::NulError) -> Self {
        Error::InteriorNul
    }
}

use std::{error, fmt};

/// The error type used by this library.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    inner: Option<Box<dyn error::Error + Send + Sync>>,
}

/// List of specific kinds of errors that may occur when using this library.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ErrorKind {
    /// A USB I/O error was encountered.
    ///
    /// This usually indicates that there is a permission problem, that a driver or another
    /// application is using the device, or that the device was unplugged.
    Usb,

    /// Multiple matching devices were found.
    MultipleDevicesFound,

    /// No matching device was found.
    NoDeviceFound,

    /// A device was opened that is incompatible with this library.
    ///
    /// This either means that `rftdi` is missing support for the device (please file an issue), or
    /// that a non-FTDI device was opened.
    UnsupportedDevice,

    /// Other errors that don't fit the other variants.
    Other,
}

impl Error {
    pub(crate) fn new(
        kind: ErrorKind,
        inner: impl Into<Box<dyn error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            kind,
            inner: Some(inner.into()),
        }
    }

    pub(crate) fn usb(inner: rusb::Error) -> Self {
        Self {
            kind: ErrorKind::Usb,
            inner: Some(Box::new(inner)),
        }
    }

    pub(crate) fn from_kind(kind: ErrorKind) -> Self {
        Self { kind, inner: None }
    }

    /// Returns the `ErrorKind` most closely describing this error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self.kind {
            ErrorKind::Usb => "USB error",
            ErrorKind::MultipleDevicesFound => "multiple matching devices found",
            ErrorKind::NoDeviceFound => "no matching devices found",
            ErrorKind::UnsupportedDevice => "device is not supported by rftdi",
            ErrorKind::Other => "other error",
        };

        // FIXME: I think you're not supposed to print the inner error?
        match &self.inner {
            Some(inner) => write!(f, "{}: {}", msg, inner),
            None => f.write_str(msg),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.inner.as_ref().map(|e| &**e as &dyn error::Error)
    }
}

use std::{error, fmt};

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    inner: Option<Box<dyn error::Error + Send + Sync>>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    Usb,
    MultipleDevicesFound,
    NoDeviceFound,
    UnsupportedDevice,
    Other,
}

impl Error {
    pub(crate) fn other(inner: impl Into<Box<dyn error::Error + Send + Sync>>) -> Self {
        Self {
            kind: ErrorKind::Other,
            inner: Some(inner.into()),
        }
    }
}

impl From<rusb::Error> for Error {
    fn from(e: rusb::Error) -> Self {
        Self {
            kind: ErrorKind::Usb,
            inner: Some(Box::new(e)),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self { kind, inner: None }
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

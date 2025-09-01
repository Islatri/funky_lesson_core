pub type Result<T> = core::result::Result<T, Error>;

pub struct Error {
    pub inner: Box<ErrorKind>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Error {
        Error {
            inner: Box::new(kind),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error::new(kind)
    }
}

#[cfg(feature = "no-wasm")]
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::new(ErrorKind::ReqwestError(e))
    }
}

#[cfg(feature = "wasm")]
impl From<gloo_net::Error> for Error {
    fn from(e: gloo_net::Error) -> Error {
        Error::new(ErrorKind::CourseError(e.to_string()))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::new(ErrorKind::SerdeJsonError(e))
    }
}

impl From<base64_simd::Error> for Error {
    fn from(e: base64_simd::Error) -> Error {
        Error::new(ErrorKind::Base64Error(e))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::new(ErrorKind::StdIoError(e))
    }
}

pub enum ErrorKind {
    #[cfg(feature = "no-wasm")]
    ReqwestError(reqwest::Error),
    #[cfg(feature = "wasm")]
    GlooNetError(gloo_net::Error),
    SerdeJsonError(serde_json::Error),
    Base64Error(base64_simd::Error),
    StdIoError(std::io::Error),
    ParseError(String),
    CourseError(String),
}

impl std::fmt::Debug for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            #[cfg(feature = "no-wasm")]
            ErrorKind::ReqwestError(ref e) => write!(f, "ReqwestError: {e:?}"),
            #[cfg(feature = "wasm")]
            ErrorKind::GlooNetError(ref e) => write!(f, "GlooNetError: {:?}", e),
            ErrorKind::SerdeJsonError(ref e) => write!(f, "SerdeJsonError: {e:?}"),
            ErrorKind::Base64Error(ref e) => write!(f, "Base64Error: {e:?}"),
            ErrorKind::StdIoError(ref e) => write!(f, "StdIoError: {e:?}"),
            ErrorKind::ParseError(ref e) => write!(f, "ParseError: {e:?}"),
            ErrorKind::CourseError(ref e) => write!(f, "CourseError: {e:?}"),
        }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            #[cfg(feature = "no-wasm")]
            ErrorKind::ReqwestError(ref e) => write!(f, "ReqwestError: {e:?}"),
            #[cfg(feature = "wasm")]
            ErrorKind::GlooNetError(ref e) => write!(f, "GlooNetError: {:?}", e),
            ErrorKind::SerdeJsonError(ref e) => write!(f, "SerdeJsonError: {e:?}"),
            ErrorKind::Base64Error(ref e) => write!(f, "Base64Error: {e:?}"),
            ErrorKind::StdIoError(ref e) => write!(f, "StdIoError: {e:?}"),
            ErrorKind::ParseError(ref e) => write!(f, "ParseError: {e:?}"),
            ErrorKind::CourseError(ref e) => write!(f, "CourseError: {e:?}"),
        }
    }
}

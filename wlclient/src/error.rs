
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // clients errors
    NoSuchObject,
    HandlerAlreadlyInPlace,
    InvalidInterface,

    // other modules errors
    WlProtocolError(crate::protocol::Error),
    Context { error: Box<Error>, message: String },
    FallBack(Box<dyn std::error::Error>),
}

// https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md
impl<T> From<T> for Error
where
    T: std::error::Error + 'static,
{
    default fn from(value: T) -> Self {
        Error::FallBack(Box::new(value))
    }
}


impl From<protocol::Error> for Error {
    fn from(value: protocol::Error) -> Self {
        Error::WlProtocolError(value)
    }
}

macro_rules! fallback_error {
    ($($t : tt)*) => { crate::error::Error::FallBack(format!($($t)*).into()) }
}

//macro_rules! fatal_error {
//    ($($t : tt)*) => { crate::error::Error::Fatal(format!($($t)*)) }
//}

macro_rules! error_context{
    ($result : expr, $($t : tt)*) => {
        $result.map_err(|e| crate::error::Error::Context {
            error   : Box::new(e.into()),
            message : format!($($t)*)
        })
    };
}

pub(super) use error_context;
pub(super) use fallback_error;

use crate::protocol;
//pub(super) use fatal_error;

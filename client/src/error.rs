use std::io::Error as IoError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoError(IoError), // TODO: look at this sometime
    Wrapper { error: Box<Error>, message: String },
    Fatal(String),
    FallBack(String),
}

//impl Error {
//    // FIXME: Find another way to wrap fatal into errors c:
//    pub fn is_fatal(&self) -> bool {
//        match self {
//            Self::Fatal(_) => true,
//            Self::Wrapper { error, .. } => error.is_fatal(),
//            _ => false,
//        }
//    }
//}

// https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md
impl<T> From<T> for Error
where
    T: std::fmt::Display,
{
    default fn from(value: T) -> Self {
        Error::FallBack(format!("Error {value}"))
    }
}

impl From<IoError> for Error {
    fn from(value: IoError) -> Self {
        Error::IoError(value)
    }
}

macro_rules! fallback_error {
    ($($t : tt)*) => { crate::error::Error::FallBack(format!($($t)*)) }
}

//macro_rules! fatal_error {
//    ($($t : tt)*) => { crate::error::Error::Fatal(format!($($t)*)) }
//}

macro_rules! error_context{
    ($result : expr, $($t : tt)*) => {
        $result.map_err(|e| crate::error::Error::Wrapper {
            error   : Box::new(e.into()),
            message : format!($($t)*)
        })
    };

    (@debug = $result : expr, $($t : tt)*) => {
        error_context!(
            $result.map_err(|e| {
                 crate::error::Error::FallBack(format!("{:?}", e))
            }),
            $($t)*
        )
    }
}

pub(super) use error_context;
pub(super) use fallback_error;
//pub(super) use fatal_error;

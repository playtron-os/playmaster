use std::fmt::Debug;

pub type EmptyResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
pub type ResultWithError<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub trait ResultTrait<T, E> {
    fn auto_err(self, desc: &str) -> ResultWithError<T>;
}

impl<T, E> ResultTrait<T, E> for Result<T, E>
where
    E: Debug,
{
    fn auto_err(self, desc: &str) -> ResultWithError<T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(format!("{desc}: {e:?}").into()),
        }
    }
}

pub trait OptionResultTrait<T> {
    fn auto_err(self, desc: &str) -> ResultWithError<T>;
}

impl<T> OptionResultTrait<T> for Option<T> {
    fn auto_err(self, desc: &str) -> ResultWithError<T> {
        match self {
            Some(t) => Ok(t),
            None => Err(format!("{desc}: None Option").into()),
        }
    }
}

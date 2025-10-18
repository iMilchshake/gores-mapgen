use std::convert::TryInto;
use std::fmt;

pub trait To {
    fn to<T>(self) -> T
    where
        Self: Into<T> + Sized,
    {
        self.into()
    }
}

impl<T> To for T {}

pub trait TryTo {
    fn try_to<T>(self) -> T
    where
        Self: TryInto<T> + Sized,
        <Self as TryInto<T>>::Error: fmt::Debug,
    {
        self.try_into().unwrap()
    }
}

impl<T> TryTo for T {}

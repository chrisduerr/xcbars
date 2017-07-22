#[macro_use]
mod composite;

macro_rules! try_xcb {
    ($func:expr, $error:expr, $($args:expr),*) => {
        $func($($args),*)
            .request_check()
            .map_err(|e| $crate::error::Error::with_chain($crate::error::Error::from(e), $error))?;
    }
}

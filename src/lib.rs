#![allow(dead_code)]

mod error;

#[cfg(unix)]
mod unix;

pub mod prelude {
    use super::*;
    pub use error::*;

    #[cfg(unix)]
    pub use unix::*;
}


#![recursion_limit="256"]

#[macro_use] pub extern crate quote;
#[macro_use] extern crate bitflags;
pub extern crate syn;
pub extern crate proc_macro2;
pub extern crate proc_macro2_diagnostics;


#[macro_use] mod macros;
#[macro_use] pub mod mapper;
#[macro_use] pub mod validator;
mod field;
mod generator;
mod support;
mod derived;
mod from_meta;

pub mod ext;

pub use crate::field::*;
pub use crate::support::Support;
pub use crate::generator::*;
pub use crate::from_meta::*;
pub use crate::derived::*;
pub use proc_macro2_diagnostics::{Diagnostic, Level};
pub use syn::spanned::Spanned;
pub use crate::mapper::{Mapper, MapperBuild};
pub use crate::validator::{Validator, ValidatorBuild};

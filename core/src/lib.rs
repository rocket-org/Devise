#![recursion_limit = "256"]

#[macro_use]
pub extern crate quote;
#[macro_use]
extern crate bitflags;
pub extern crate proc_macro2;
pub extern crate proc_macro2_diagnostics;
pub extern crate syn;

#[macro_use]
mod macros;
#[macro_use]
pub mod mapper;
#[macro_use]
pub mod validator;
mod derived;
mod field;
mod from_meta;
mod generator;
mod support;

pub mod ext;

pub use derived::*;
pub use field::*;
pub use from_meta::*;
pub use generator::*;
pub use mapper::{Mapper, MapperBuild};
pub use proc_macro2_diagnostics::{Diagnostic, Level};
pub use support::Support;
pub use syn::spanned::Spanned;
pub use validator::{Validator, ValidatorBuild};

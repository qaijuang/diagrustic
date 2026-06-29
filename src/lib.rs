#![cfg_attr(not(feature = "std"), no_std)]
#![feature(allocator_api, clone_from_ref)]
#![warn(clippy::pedantic)]

extern crate alloc;

mod acow;
mod emit;
mod span;
#[cfg(feature = "std")]
mod styles;
pub mod sub_diag;
#[cfg(feature = "std")]
mod sys;

pub mod applicability;
pub mod builder;
pub mod diagnostic;
pub mod level;
pub mod source_map;
pub mod suggestion;

pub use acow::{Acow, IntoAcow};
pub use emit::EmitDiagnostic;
#[cfg(feature = "std")]
pub use emit::terminal::{ColorChoice, DiagnosticFormat, EmitterConfig, TerminalEmitter};

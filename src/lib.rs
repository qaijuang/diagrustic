#![cfg_attr(not(feature = "std"), no_std)]
#![feature(allocator_api)]
#![warn(clippy::pedantic)]

extern crate alloc;

pub mod applicability;
pub mod builder;
pub mod diagnostic;
mod emit;
pub mod level;
pub mod source_map;
pub mod span;
#[cfg(feature = "std")]
mod styles;
pub mod sub_diag;
pub mod suggestion;
#[cfg(feature = "std")]
mod sys;

pub use emit::EmitDiagnostic;
#[cfg(feature = "std")]
pub use emit::terminal::{ColorChoice, DiagnosticFormat, EmitterConfig, TerminalEmitter};

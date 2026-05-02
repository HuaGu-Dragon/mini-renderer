#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(feature = "derive")]
pub use mini_renderer_derive::Varying;

pub mod graphics;
pub mod math;
pub mod pipeline;
pub mod renderer;

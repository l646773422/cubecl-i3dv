mod base;
#[cfg(target_os = "windows")]
pub(crate) mod compilation_worker;
mod wgsl;

#[cfg(feature = "spirv")]
pub mod vulkan;

#[cfg(all(feature = "msl", target_os = "macos"))]
pub mod metal;

pub use base::*;

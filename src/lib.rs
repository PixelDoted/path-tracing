#![feature(f16)]
mod bvh;
pub mod data;
pub mod shader;

pub use data::RayTraceSettings;
pub use shader::RayTracePlugin;

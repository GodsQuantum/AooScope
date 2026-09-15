#![forbid(unsafe_code)]

mod compiler;
mod geometry;
mod media;
mod revisions;
pub mod typography;

pub use compiler::{
    CompileError, CompiledDocument, CompiledPage, HEIGHT, WIDTH, compile_document, compile_page,
};
pub use media::{MediaError, MediaStore};
pub use revisions::{RevisionError, RevisionStore};

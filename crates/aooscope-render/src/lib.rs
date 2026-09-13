#![forbid(unsafe_code)]

mod compiler;
mod media;
mod revisions;

pub use compiler::{
    CompileError, CompiledDocument, CompiledPage, HEIGHT, WIDTH, compile_document, compile_page,
};
pub use media::{MediaError, MediaStore};
pub use revisions::{RevisionError, RevisionStore};

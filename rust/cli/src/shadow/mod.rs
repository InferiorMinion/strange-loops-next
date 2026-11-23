//! Read and preflight AD2 imports. Publication is the next CLI piece.
#![expect(
    dead_code,
    reason = "The shadow-import verb wires in this preflight in piece C."
)]

mod mapping;
mod source;

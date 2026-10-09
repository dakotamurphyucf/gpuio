//! Export the pinned two-face embedded-data acknowledgements without a GUI.
//! This is an upstream-provided subset, not full dependency/license approval.
use std::io::{self, Write};

fn main() -> io::Result<()> {
    io::stdout()
        .lock()
        .write_all(two_face::acknowledgement::listing().to_md().as_bytes())
}

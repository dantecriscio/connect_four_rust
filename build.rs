//! Unzips the checked-in opening books (openingBook.zip) into unzipped_opening_books/ before building.
//!
//! Only books that are missing, or older than their copy in the zip, are extracted,
//! so books generated locally by book-maker are never overwritten.
//! Uses the system `unzip` tool. Failures are reported as warnings rather than errors,
//! because the solver still builds and runs without an opening book.

use std::path::Path;
use std::process::Command;

const ZIP: &str = "openingBook.zip";
const BOOK_DIR: &str = "unzipped_opening_books";

fn main() {
    // Rerun when the zip changes, or when anything in the book directory changes (e.g. deleted)
    println!("cargo::rerun-if-changed={ZIP}");
    println!("cargo::rerun-if-changed={BOOK_DIR}");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo");
    let zip = Path::new(&manifest_dir).join(ZIP);
    let book_dir = Path::new(&manifest_dir).join(BOOK_DIR);

    if !zip.exists() {
        println!("cargo::warning={ZIP} not found, so no opening books were unzipped");
        return;
    }

    // -u: only extract files that are missing or older than the zip's copy
    // -o: do not prompt before replacing those older files
    // -q: quiet
    let result = Command::new("unzip")
        .arg("-uoq")
        .arg(&zip)
        .arg("-d")
        .arg(&book_dir)
        .status();

    match result {
        Ok(status) if status.success() => {}
        Ok(status) => println!("cargo::warning=unzip of {ZIP} failed with {status}"),
        Err(error) => println!("cargo::warning=could not run unzip for {ZIP}: {error}"),
    }
}

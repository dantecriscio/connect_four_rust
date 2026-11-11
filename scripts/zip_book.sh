#!/usr/bin/env bash
# Zips every book in unzipped_opening_books/ into openingBook.zip (replacing it), for checking into git.
# `cargo build` unzips it again via build.rs.
#
# Usage: scripts/zip_book.sh
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BOOK_DIR="$REPO_DIR/unzipped_opening_books"
ZIP="$REPO_DIR/openingBook.zip"

shopt -s nullglob
books=("$BOOK_DIR"/depth_*.bin)
if (( ${#books[@]} == 0 )); then
    echo "No books found in $BOOK_DIR" >&2
    exit 1
fi

# Build to a temporary file, so a failure never leaves a half-written zip in place
rm -f "$ZIP.tmp"
# -j: store bare file names, -X: omit OS-specific extra attributes, -9: best compression
zip -j -X -9 "$ZIP.tmp" "${books[@]}"
mv "$ZIP.tmp" "$ZIP"

echo "Wrote ${#books[@]} books to $ZIP ($(wc -c < "$ZIP" | tr -d ' ') bytes)"

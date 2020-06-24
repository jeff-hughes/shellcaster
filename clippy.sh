#!/bin/bash
# I prefer to be verbose, Rust, kthx
touch src/main.rs && cargo clippy -- -A clippy::redundant_field_names -A clippy::needless_return
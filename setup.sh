#!/usr/bin/env bash

cd shell-scriptman
cargo build --release
ln -s "$(pwd)/she/target/release/shell-scriptman" ../cmd

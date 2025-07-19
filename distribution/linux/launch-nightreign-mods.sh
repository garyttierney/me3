#!/bin/bash

script_dir=$(dirname "$(realpath "$0")")
"$script_dir/bin/me3" \
    --windows-binaries-dir "$script_dir/bin/win64" \
    launch --auto-detect \
    -p "$script_dir/nightreign-default.me3"

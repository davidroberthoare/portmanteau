#!/bin/sh
# Print the one version number (Cargo.toml [workspace.package]).
awk '/^\[workspace.package\]/{ f=1; next } f && /^version *=/ { gsub(/[" ]/, "", $0); sub(/version=/, "", $0); print; exit }' "${1:-Cargo.toml}"

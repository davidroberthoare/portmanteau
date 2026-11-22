#!/bin/sh
# Print the newest section of docs/RELEASE_NOTES.md (from its first "## v" line
# up to, not including, the next one). Used for the body of a GitHub release.
awk '/^## v/{ if (seen) exit; seen=1 } seen { print }' "${1:-docs/RELEASE_NOTES.md}"

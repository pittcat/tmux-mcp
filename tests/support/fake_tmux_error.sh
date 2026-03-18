#!/bin/bash
# Fake tmux that returns an error
# Used for testing error handling
echo "tmux: invalid option" >&2
exit 1
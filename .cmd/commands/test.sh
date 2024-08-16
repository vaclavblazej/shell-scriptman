#!/usr/bin/env sh

echo "----------------------------------------"
echo "Script location is $0"
echo "Working directory $(pwd)"
echo "I got $# arguments:"
for arg in "$@"; do
    echo "$arg"
done
echo "----------------------------------------"

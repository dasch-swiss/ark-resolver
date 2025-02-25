#!/bin/sh
set -e  # Exit immediately if a command exits with a non-zero status
exec python3 -m ark_resolver.ark "$@"
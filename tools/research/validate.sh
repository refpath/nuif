#!/usr/bin/env sh
# Validate research records. Requires python3 with jsonschema and pyyaml
# (pip install jsonschema pyyaml, or use a virtual environment).
set -eu
cd "$(git rev-parse --show-toplevel)"
exec "${PYTHON:-python3}" tools/research/validate.py "$@"

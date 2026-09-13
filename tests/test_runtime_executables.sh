#!/usr/bin/env bash
set -u
image=${1:?usage: $0 IMAGE}
for executable in python python3 node waitress-serve asterctl; do
  if docker run --rm --entrypoint /bin/sh "$image" -c "command -v $executable >/dev/null 2>&1"; then
    echo "FAIL: forbidden executable present: $executable"
    exit 1
  fi
done
echo "PASS: runtime executable scan ($image)"

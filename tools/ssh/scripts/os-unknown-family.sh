#!/bin/sh
# Scanopy SSH credential script: an `os` object whose family Scanopy has no variant for.
#
# Expected: hostname and model are applied; `os` is reported as invalid on the run and the host
# gets no OS from the script.
host="${SCANOPY_LAB_HOSTNAME:-$(uname -n)}"
printf '{"hostname":"%s","model":"Lab Model O","os":{"family":"Plan9","name":"Plan 9 from Bell Labs","version":"4"}}\n' \
    "$host"

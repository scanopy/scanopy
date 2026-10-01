#!/bin/sh
# Scanopy SSH credential script: exits non-zero after printing valid JSON.
#
# The output parses and names a hostname, so a daemon that applies stdout without checking the
# exit status renames the host to "applied-despite-exit-3". Expected: the run is reported as
# failed with exit status 3 and the stderr line below, and nothing is applied.
printf '{"hostname":"applied-despite-exit-3","sys_descr":"this output must not be applied"}\n'
echo "scanopy-ssh-lab: simulated failure, exiting 3" >&2
exit 3

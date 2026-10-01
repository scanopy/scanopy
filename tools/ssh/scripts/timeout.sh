#!/bin/sh
# Scanopy SSH credential script: runs past the credential timeout.
#
# Sleeps 120 s, twice the default timeout_seconds of 60, then prints valid JSON. Expected: the
# daemon gives up at the credential's timeout, reports a timeout, applies nothing, and the scan
# carries on with the other hosts. If the credential's timeout is raised above 120 s, the run
# succeeds and the hostname below is applied.
sleep 120
printf '{"hostname":"applied-after-timeout"}\n'

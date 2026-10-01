#!/bin/sh
# Scanopy SSH credential script: prints valid JSON larger than the 64 KiB stdout cap.
#
# The object is well formed (about 80 KiB, almost all of it one sys_descr value), so a daemon
# that read it in full would parse it and apply it. Expected: the daemon stops reading at
# 64 KiB, reports the output as oversized, and applies nothing.
printf '{"hostname":"applied-from-oversized","sys_descr":"'
head -c 81920 /dev/zero | tr '\000' 'A'
printf '"}\n'

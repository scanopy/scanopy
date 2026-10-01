#!/bin/sh
# Scanopy SSH credential script: valid known keys plus keys Scanopy has no field for.
#
# Expected: hostname, sys_descr, model and the interface are applied; favorite_color,
# rack_units and extra_info are reported as unknown keys and nothing else happens to them.
host="${SCANOPY_LAB_HOSTNAME:-$(uname -n)}"
addr="${SCANOPY_LAB_ADDR:-}"
ips=""
if [ -n "$addr" ]; then
    ips="\"${addr}/22\""
fi
printf '{"hostname":"%s","sys_descr":"unknown-keys fixture","model":"Lab Model U","favorite_color":"teal","rack_units":2,"extra_info":{"owner":"lab"},"interfaces":[{"name":"lab0","ips":[%s]}]}\n' \
    "$host" "$ips"

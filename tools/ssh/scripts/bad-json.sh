#!/bin/sh
# Scanopy SSH credential script: exits 0 with output that is not valid JSON.
#
# Starts like a real inventory and stops mid-array, the shape a script killed partway through
# or one with an unescaped value produces. Expected: the run is reported as a parse failure and
# no field is applied, including the hostname that precedes the break.
printf '{"hostname":"applied-from-bad-json","interfaces":[{"name":"eth0","ips":["192.168.7.163/22"'
printf '\n'
exit 0

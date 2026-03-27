#!/bin/bash
# snmp-pass-handler.sh — net-snmp pass protocol handler
# Usage: pass .1.3.6.1.2.1.2.2 /path/to/snmp-pass-handler.sh /path/to/data-file.txt
#
# Data file format (one per line, sorted by OID):
#   OID TYPE VALUE
#
# Responds to:
#   -g OID  → GET exact match
#   -n OID  → GETNEXT (next OID after given)
#
# Types: string, integer, gauge, counter, timeticks, objectid, octet

DATA_FILE="$1"
REQUEST="$2"
OID="$3"

if [ ! -f "$DATA_FILE" ]; then
    echo "NONE"
    exit 0
fi

case "$REQUEST" in
    -g)
        # GET — exact OID lookup
        LINE=$(awk -v oid="$OID" '$1 == oid { print; exit }' "$DATA_FILE")
        if [ -z "$LINE" ]; then
            echo "NONE"
            exit 0
        fi
        echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
        ;;
    -n)
        # GETNEXT — find next OID after the given one
        LINE=$(awk -v oid="$OID" '
            BEGIN { found = 0 }
            {
                if (found == 0) {
                    # Compare OIDs: find first OID that is strictly greater
                    if (oid_gt($1, oid)) {
                        print
                        exit
                    }
                }
            }
            function oid_gt(a, b,    na, nb, sa, sb, i) {
                na = split(a, sa, ".")
                nb = split(b, sb, ".")
                for (i = 1; i <= (na > nb ? na : nb); i++) {
                    ai = (i <= na) ? sa[i]+0 : -1
                    bi = (i <= nb) ? sb[i]+0 : -1
                    if (ai > bi) return 1
                    if (ai < bi) return 0
                }
                return 0
            }
        ' "$DATA_FILE")
        if [ -z "$LINE" ]; then
            echo "NONE"
            exit 0
        fi
        echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
        ;;
    *)
        echo "NONE"
        exit 0
        ;;
esac

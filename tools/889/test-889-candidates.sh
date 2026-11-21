#!/bin/sh
# Offline self-test for 889-candidates.sh. Builds a tiny feed release with
# correct manifests and checks: covered names are reported as covered, an
# 889-owned affiliate missing from the vendor list is proposed with an escaped
# pattern, an affiliate of a non-889 party is ignored, and a file that does not
# match its manifest hash fails with exit 2.
set -eu

SCRIPT_DIR=$(unset CDPATH; cd -- "$(dirname -- "$0")" && pwd)
TMP=$(mktemp -d "${TMPDIR:-/tmp}/test889c.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM
F="$TMP/feed"
mkdir -p "$F"
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }

cat >"$F/us_far_889.jsonl" <<'EOF'
{"NAME_ORG":"ZTE Corporation","NAMES":[{"NAME_TYPE":"PRIMARY","NAME_ORG":"ZTE Corporation"}],"SOURCE_DOC":"48 CFR 52.204-25"}
EOF
cat >"$F/affiliates.senzing.jsonl" <<'EOF'
{"NAME_ORG":"Example (Shenzhen) Co.; Ltd.","NAMES":[{"NAME_TYPE":"PRIMARY","NAME_ORG":"Example (Shenzhen) Co.; Ltd."}],"LEI_NUMBER":"LEI0000000000000001","OWNERSHIP_VERDICTS":{"any-list-50":"definitely"},"OWNERSHIP_PROOF":{"any-list-50":{"proof":[{"round":0,"lists":["US_FAR_889"],"name":"ZTE Corporation"},{"round":1,"via":[{"source":"GLEIF IS_DIRECTLY_CONSOLIDATED_BY","as_of":"2026-11-10"}]}]}}}
{"NAME_ORG":"ZTE Software Ltd","NAMES":[],"LEI_NUMBER":"LEI0000000000000002","OWNERSHIP_VERDICTS":{"any-list-50":"definitely"},"OWNERSHIP_PROOF":{"any-list-50":{"proof":[{"round":0,"lists":["US_FAR_889"],"name":"ZTE Corporation"}]}}}
{"NAME_ORG":"Unrelated Bank JSC","NAMES":[],"LEI_NUMBER":"LEI0000000000000003","OWNERSHIP_VERDICTS":{"ofac-50":"definitely"},"OWNERSHIP_PROOF":{"ofac-50":{"proof":[{"round":0,"lists":["SDN"],"name":"Listed Bank"}]}}}
EOF

far=$(sha256sum "$F/us_far_889.jsonl" | cut -d' ' -f1)
aff=$(sha256sum "$F/affiliates.senzing.jsonl" | cut -d' ' -f1)
printf '{"built_at":"2026-11-18T02:14:50Z","lists":{"us_far_889":{"sha256":"%s","notes":{"ecfr_as_of":"2026-11-16"}}}}\n' "$far" >"$F/manifest.json"
printf '{"data_date":"2026-11-18","stale":false,"files":{"affiliates.senzing.jsonl":{"sha256":"%s"}},"inputs":{"us_far_889":{"sha256":"%s"}}}\n' "$aff" "$far" >"$F/affiliates.manifest.json"

out="$TMP/out.md"
"$SCRIPT_DIR/889-candidates.sh" --dir "$F" --vendors "$SCRIPT_DIR/889-vendors.txt" >"$out" \
    || fail "clean feed did not exit 0"

grep -q '| named | ZTE Corporation |.*covered by' "$out" || fail "named party not reported as covered"
grep -q '| affiliate | ZTE Software Ltd |.*covered by' "$out" || fail "affiliate matched by an existing pattern not reported as covered"
grep -qF 'example \(shenzhen\) co\.. ltd\.;us-restricted-parties;2026-11-18;affiliate ZTE Corporation (LEI LEI0000000000000001' "$out" \
    || fail "uncovered affiliate not proposed with an escaped pattern"
grep -q 'Unrelated Bank' "$out" && fail "affiliate of a non-889 party was reported"
grep -q '3 name(s) checked.*1 not covered' "$out" || fail "summary counts wrong"

# The proposed pattern must match the name it was made from.
pat=$(grep -F ';us-restricted-parties;' "$out" | head -n1 | cut -d';' -f1)
printf '%s' "Example (Shenzhen) Co.; Ltd." | grep -i -E -q -- "$pat" \
    || fail "proposed pattern does not match its own name: $pat"

# A file that does not match its manifest hash is rejected.
printf '\n' >>"$F/affiliates.senzing.jsonl"
rc=0
"$SCRIPT_DIR/889-candidates.sh" --dir "$F" >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 2 ] || fail "hash mismatch exited $rc, want 2"

printf 'test-889-candidates.sh: PASS\n'

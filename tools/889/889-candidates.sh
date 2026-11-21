#!/bin/sh
# ==============================================================================
# 889-candidates.sh — vendor-list review aid for the quarterly 889 refresh
# ==============================================================================
#
# Pulls one dated release of the us-restricted-parties data feed, checks every
# file it reads against the SHA-256 in that release's own manifest, and reports
# which Section 889 parties and ownership-derived affiliates are NOT yet matched
# by any pattern in 889-vendors.txt. Prints a Markdown report for the quarterly
# review issue, with ready-to-paste `pattern;source;date;note` lines.
#
# It never edits 889-vendors.txt and the matcher (check-889.sh) stays offline:
# a maintainer reviews the report and decides what to add.
#
# Feed: https://github.com/Mattbusel/us-restricted-parties (CC BY 4.0, rebuilt
# daily in public CI from eCFR FAR 52.204-25, the FCC Covered List and other
# lists; affiliates come from GLEIF Level 2 ownership links).
#
# Dependencies: POSIX shell + `jq` + `curl` + `sha256sum`.
#
# ------------------------------------------------------------------------------
# USAGE
# ------------------------------------------------------------------------------
#   889-candidates.sh [OPTIONS]
#
# OPTIONS
#   --release TAG    Feed release to read (e.g. data-2026-11-18). Default: the
#                    newest data-* release. The tag is printed in the report.
#   --dir DIR        Read the feed files from DIR instead of downloading them
#                    (offline; used by the self-test).
#   --vendors FILE   Vendor pattern list (default: 889-vendors.txt next to this
#                    script).
#   -h, --help       Show this help and exit.
#
# EXIT STATUS
#   0   Report written (whether or not there are uncovered candidates).
#   2   Usage error, missing dependency, download failure or hash mismatch.
# ==============================================================================

set -eu

PROG="889-candidates.sh"
FEED_REPO="Mattbusel/us-restricted-parties"
SCRIPT_DIR=$(unset CDPATH; cd -- "$(dirname -- "$0")" && pwd)
VENDORS="$SCRIPT_DIR/889-vendors.txt"
RELEASE=""
DIR=""
FILES="manifest.json affiliates.manifest.json us_far_889.jsonl affiliates.senzing.jsonl"

usage() { sed -n '/^# USAGE$/,/^# ===/p' "$0" | sed 's/^# \{0,1\}//; s/^#$//'; }
err() { printf '%s: %s\n' "$PROG" "$1" >&2; }

while [ $# -gt 0 ]; do
    case "$1" in
        --release) [ $# -ge 2 ] || { err "--release needs a value"; exit 2; }; RELEASE="$2"; shift 2 ;;
        --release=*) RELEASE="${1#*=}"; shift ;;
        --dir) [ $# -ge 2 ] || { err "--dir needs a value"; exit 2; }; DIR="$2"; shift 2 ;;
        --dir=*) DIR="${1#*=}"; shift ;;
        --vendors) [ $# -ge 2 ] || { err "--vendors needs a value"; exit 2; }; VENDORS="$2"; shift 2 ;;
        --vendors=*) VENDORS="${1#*=}"; shift ;;
        -h|--help) usage; exit 0 ;;
        *) err "unknown argument: $1"; exit 2 ;;
    esac
done

for tool in jq sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || { err "$tool is required but not found on PATH"; exit 2; }
done
[ -f "$VENDORS" ] || { err "vendor list not found: $VENDORS"; exit 2; }

US=$(printf '\037')
TMP=$(mktemp -d "${TMPDIR:-/tmp}/candidates889.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM

# ---- fetch -------------------------------------------------------------------
if [ -z "$DIR" ]; then
    command -v curl >/dev/null 2>&1 || { err "curl is required but not found on PATH"; exit 2; }
    if [ -z "$RELEASE" ]; then
        RELEASE=$(curl -fsSL "https://api.github.com/repos/$FEED_REPO/releases?per_page=30" \
            | jq -r '[.[] | select(.draft | not) | .tag_name | select(startswith("data-"))][0] // empty') \
            || { err "could not list feed releases"; exit 2; }
        [ -n "$RELEASE" ] || { err "no data-* release found in $FEED_REPO"; exit 2; }
    fi
    DIR="$TMP/feed"
    mkdir -p "$DIR"
    for f in $FILES; do
        curl -fsSL -o "$DIR/$f" "https://github.com/$FEED_REPO/releases/download/$RELEASE/$f" \
            || { err "download failed: $RELEASE/$f"; exit 2; }
    done
fi
for f in $FILES; do
    [ -f "$DIR/$f" ] || { err "missing feed file: $DIR/$f"; exit 2; }
done
[ -n "$RELEASE" ] || RELEASE="(local: $DIR)"

# ---- verify ------------------------------------------------------------------
# Each data file must hash to what its release manifest says, and the affiliates
# build must have been made from this same us_far_889.jsonl.
sha() { sha256sum "$1" | cut -d' ' -f1; }
check() {
    # $1 file  $2 expected sha256
    got=$(sha "$DIR/$1")
    [ -n "$2" ] && [ "$got" = "$2" ] || { err "sha256 mismatch for $1: manifest ${2:-<none>}, got $got"; exit 2; }
}
far_sha=$(jq -r '.lists.us_far_889.sha256 // empty' "$DIR/manifest.json")
aff_sha=$(jq -r '.files["affiliates.senzing.jsonl"].sha256 // .sha256 // empty' "$DIR/affiliates.manifest.json")
aff_in=$(jq -r '.inputs.us_far_889.sha256 // empty' "$DIR/affiliates.manifest.json")
check us_far_889.jsonl "$far_sha"
check affiliates.senzing.jsonl "$aff_sha"
[ "$aff_in" = "$far_sha" ] || { err "affiliates were built from a different us_far_889.jsonl"; exit 2; }

built_at=$(jq -r '.built_at // ""' "$DIR/manifest.json")
list_date=${built_at%%T*}
aff_date=$(jq -r '.data_date // (.built_at // "" | split("T")[0])' "$DIR/affiliates.manifest.json")
aff_stale=$(jq -r '.stale // false' "$DIR/affiliates.manifest.json")
ecfr=$(jq -r '.lists.us_far_889.notes.ecfr_as_of // ""' "$DIR/manifest.json")
stale_note=""
[ "$aff_stale" != "true" ] || stale_note=" (**stale**: the feed reused an older affiliates build)"

# ---- candidates --------------------------------------------------------------
# One row per name: kind, name, parent/owner, note, date.
CAND="$TMP/candidates"
jq -r --arg us "$US" --arg d "$list_date" '
    . as $r | ([$r.NAME_ORG] + [$r.NAMES[]?.NAME_ORG]) | map(select(. != null)) | unique | .[]
    | ["named", ., $r.NAME_ORG, ($r.SOURCE_DOC // "FAR 52.204-25"), $d] | join($us)
' "$DIR/us_far_889.jsonl" >"$CAND"
jq -r --arg us "$US" --arg d "$aff_date" '
    select(any(.OWNERSHIP_PROOF[]?.proof[]? | select(.round == 0) | .lists[]?; . == "US_FAR_889"))
    | . as $r
    | ([.OWNERSHIP_PROOF[]?.proof[]? | select(.round == 0 and any(.lists[]?; . == "US_FAR_889")) | .name] | unique | join(", ")) as $owner
    | ([.OWNERSHIP_PROOF[]?.proof[]? | .via[]? | .source] | unique | join(", ")) as $edge
    | ([.OWNERSHIP_PROOF[]?.proof[]? | .via[]? | .as_of // empty] | max // "") as $asof
    | ([.OWNERSHIP_VERDICTS | to_entries[] | "\(.key)=\(.value)"] | join(" ")) as $verdicts
    | ([$r.NAME_ORG] + [$r.NAMES[]?.NAME_ORG]) | map(select(. != null)) | unique | .[]
    | ["affiliate", ., $owner,
       "LEI \($r.LEI_NUMBER // "?"), \($edge) as of \($asof), \($verdicts)", $d] | join($us)
' "$DIR/affiliates.senzing.jsonl" >>"$CAND"

PAT="$TMP/patterns"
: >"$PAT"
while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in ''|'#'*) continue ;; esac
    p=${line%%;*}
    [ -n "$p" ] && printf '%s\n' "$p" >>"$PAT"
done <"$VENDORS"

covered_by() {
    while IFS= read -r p; do
        if printf '%s' "$1" | grep -i -E -q -- "$p" 2>/dev/null; then
            printf '%s' "$p"; return 0
        fi
    done <"$PAT"
    return 1
}
# Field-safe text: no ';' in vendor-file fields, '|' escaped in Markdown cells.
field() { printf '%s' "$1" | tr ';' ','; }
cell() { printf '%s' "$1" | sed 's/|/\\|/g'; }
ere() { printf '%s' "$1" | tr 'A-Z' 'a-z' | sed 's/[][\\.^$*+?(){}|/]/\\&/g; s/;/./g'; }

ROWS="$TMP/rows"
NEW="$TMP/new"
: >"$ROWS"
: >"$NEW"
ncand=0
nnew=0
while IFS="$US" read -r kind name owner note date; do
    ncand=$((ncand + 1))
    if pat=$(covered_by "$name"); then
        status="covered by \`$(cell "$pat")\`"
    else
        status="**not covered**"
        nnew=$((nnew + 1))
        printf '%s;us-restricted-parties;%s;%s %s (%s)\n' \
            "$(ere "$name")" "$date" "$kind" "$(field "$owner")" "$(field "$note")" >>"$NEW"
    fi
    printf '| %s | %s | %s | %s | %s |\n' \
        "$kind" "$(cell "$name")" "$(cell "$owner")" "$(cell "$note")" "$status" >>"$ROWS"
done <"$CAND"

# ---- report ------------------------------------------------------------------
cat <<EOF
### Feed check: us-restricted-parties \`$RELEASE\`

- Section 889 parties from eCFR FAR 52.204-25${ecfr:+ (as of $ecfr)}, lists built $list_date.
- Affiliates owned or controlled by those parties from GLEIF ownership links, data date $aff_date$stale_note.
- SHA-256 of \`us_far_889.jsonl\` and \`affiliates.senzing.jsonl\` matched the release manifests.
- $ncand name(s) checked against $(wc -l <"$PAT" | tr -d ' ') pattern(s) in \`tools/889/889-vendors.txt\`; $nnew not covered.

Affiliate rows are ownership evidence, not a determination. Check each before adding it.

| kind | name | listed party | evidence | 889-vendors.txt |
|------|------|--------------|----------|-----------------|
EOF
cat "$ROWS"
if [ "$nnew" -gt 0 ]; then
    printf '\nCandidate lines for `tools/889/889-vendors.txt` (review, then edit the pattern and source as needed):\n\n```\n'
    cat "$NEW"
    printf '```\n'
fi

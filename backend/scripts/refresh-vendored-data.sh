#!/usr/bin/env bash
# Refresh the vendored data files embedded into the server binary:
#   backend/assets/{oui,mam,oui36,iab}.csv       IEEE MAC-address registries
#   backend/assets/disposable-email-domains.txt  sign-up disposable-domain list
#
# File drop only: all parsing happens at load time in
# backend/src/server/shared/oui.rs and backend/src/server/auth/disposable_email.rs.
#
# Invoked by `make refresh-vendored-data` and by the release workflow
# (.github/workflows/release.yml, test-and-update-fixtures job), which
# commits the refreshed file back to the release branch after `cargo test`.
#
# Failure semantics: best-effort. A failed download or a result below its
# sanity floor keeps the committed copy and warns; the script only exits
# non-zero on environment errors (missing curl).
#
# Requires: curl, jq.

set -u
cd "$(dirname "$0")/.." || exit 1

CURL=(curl -sfL --max-time 300 --retry 5 --retry-delay 70)
UA="ScanopyVendoredDataRefresh/1.0 (release workflow)"

command -v curl >/dev/null || { echo "ERROR: curl is required" >&2; exit 1; }
command -v jq >/dev/null || { echo "ERROR: jq is required" >&2; exit 1; }

# install <tmp-file> <dest> <line-floor> — move tmp into place if it clears
# the sanity floor, else keep the committed copy.
install_if_sane() {
    local tmp="$1" dest="$2" floor="$3" lines
    lines=$(wc -l < "$tmp" | tr -d ' ')
    if [ "$lines" -ge "$floor" ]; then
        mv "$tmp" "$dest"
        echo "refreshed $dest ($lines lines)"
    else
        echo "WARN: $dest refresh has $lines lines (floor $floor) — keeping committed copy" >&2
        rm -f "$tmp"
    fi
}

# --- IEEE MAC registries (MAC-address vendor lookup) ---
# IEEE sits behind bot protection that instantly rejects default-UA curl from
# datacenter IPs (seen as a ~300ms failure on GitHub runners) — always send
# the UA, and report HTTP status + curl exit on failure so the release log
# says exactly why a refresh was skipped.
#
# refresh_registry <url-path> <dest> <line-floor>
refresh_registry() {
    local path="$1" dest="$2" floor="$3" tmp http_code curl_exit
    tmp=$(mktemp)
    http_code=$("${CURL[@]}" -A "$UA" -o "$tmp" -w '%{http_code}' \
        "https://standards-oui.ieee.org/$path")
    curl_exit=$?
    if [ "$curl_exit" = 0 ] && head -1 "$tmp" | grep -q "Registry"; then
        install_if_sane "$tmp" "$dest" "$floor"
    elif [ "$curl_exit" != 0 ]; then
        echo "WARN: $dest download failed (curl exit $curl_exit, HTTP ${http_code:-?}) — keeping committed copy" >&2
        rm -f "$tmp"
    else
        echo "WARN: $dest malformed (HTTP $http_code, starts: $(head -c 60 "$tmp" | tr -d '\n')) — keeping committed copy" >&2
        rm -f "$tmp"
    fi
}

# MA-L (24-bit), MA-M (28-bit), MA-S (36-bit) and the retired IAB (36-bit)
# blocks. oui.rs looks up the longest prefix first. CIDs are skipped: they sit
# in locally administered space and never name a vendor.
refresh_registry oui/oui.csv assets/oui.csv 20000
refresh_registry oui28/mam.csv assets/mam.csv 4000
refresh_registry oui36/oui36.csv assets/oui36.csv 4000
refresh_registry iab/iab.csv assets/iab.csv 3000

# --- Disposable email domains (sign-up check) ---
# Pinned to upstream's current commit, which the file header records. A domain
# flagged by mistake goes in assets/disposable-email-allowlist.txt, which this
# script never touches.
refresh_disposable_domains() {
    local upstream="disposable/disposable-email-domains"
    local dest="assets/disposable-email-domains.txt"
    local commit sha date tmp
    if ! commit=$("${CURL[@]}" -A "$UA" "https://api.github.com/repos/$upstream/commits/master") \
        || ! sha=$(jq -er .sha <<<"$commit") \
        || ! date=$(jq -er .commit.committer.date <<<"$commit"); then
        echo "WARN: $dest upstream commit lookup failed — keeping committed copy" >&2
        return
    fi
    tmp=$(mktemp)
    {
        echo "# Disposable email domains, refused at Scanopy Cloud sign-up alongside the mailchecker crate."
        echo "# Source: https://github.com/$upstream (domains.txt)"
        echo "# Upstream commit: $sha ($date)"
        echo "# License: MIT. Copyright (c) 2017 Andrei Simionescu; Copyright (c) 2017 Stefan Meinecke, greenSec GmbH."
        echo "# backend/scripts/refresh-vendored-data.sh rewrites this file at each release. Add a domain"
        echo "# flagged by mistake to disposable-email-allowlist.txt instead of editing it here."
    } > "$tmp"
    if ! "${CURL[@]}" -A "$UA" "https://raw.githubusercontent.com/$upstream/$sha/domains.txt" >> "$tmp"; then
        echo "WARN: $dest download failed — keeping committed copy" >&2
        rm -f "$tmp"
        return
    fi
    install_if_sane "$tmp" "$dest" 50000
}

refresh_disposable_domains

echo "done"

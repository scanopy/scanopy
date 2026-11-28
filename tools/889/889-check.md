# NDAA Section 889 supply-chain check

Automated evidence that Scanopy's dependency tree and built container images
contain no components originating with entities prohibited under **NDAA Section
889** (Huawei, ZTE, Hytera, Hikvision, Dahua) or their subsidiaries / affiliates.
The output is supporting evidence for a Section 889 compliance attestation.

## Pieces

| File | Role |
|------|------|
| `tools/889/check-889.sh` | Matcher. POSIX shell + `jq`, no network. Scans CycloneDX SBOM(s), fails on a hit. |
| `tools/889/889-evidence.sh` | On-demand evidence bundler. Generates SBOMs + runs the matcher + emits a hash-anchored evidence bundle. |
| `tools/889/889-vendors.txt` | Maintained prohibited-vendor pattern list (single source of truth). |
| `tools/889/889-allow.txt` | Reviewed false-positive exceptions (only subtracts, never broadens). |
| `.github/workflows/889-check.yml` | PR gate — analyze-only source SBOM, blocks merge on a hit. |
| `.github/workflows/889-evidence.yml` | Refreshes `compliance/ndaa-889/` (the published evidence) and commits it to `main`. |
| `.github/workflows/889-vendors-refresh.yml` | Quarterly cron — opens a vendor-list review issue with a feed check. |
| `tools/889/889-candidates.sh` | Review aid. Reports 889 parties and ownership-derived affiliates from a hash-checked feed release that no vendor pattern covers yet. Never edits the list. |
| `.github/workflows/release.yml` (`supply-chain-889` job) | Release gate — scans source + all images, fails the release on a hit. |

## Running locally

```sh
# Source tree. Pretty-print so the matcher reports meaningful file:line.
syft scan dir:. -o cyclonedx-json | jq . > sbom-source.cdx.json
./tools/889/check-889.sh sbom-source.cdx.json

# A built image (syft pulls from the registry directly, no docker daemon needed).
syft scan registry:ghcr.io/scanopy/scanopy/server:latest -o cyclonedx-json | jq . > server.cdx.json
./tools/889/check-889.sh --json server.cdx.json
```

`syft` emits minified single-line CycloneDX by default; pipe through `jq .` so
`file:line` in hit output points at the offending component. The matcher falls
back to line 1 on minified input. `./tools/889/check-889.sh --help` documents all
options. Exit codes: `0` clean, `1` hit found, `2` usage/dependency error.

## Review evidence (the single link a signed letter cites)

The Section 889 **attestation** is the signed letter to the customer. This repo
produces the **supporting evidence** that letter points to — proof an automated
covered-entity review was performed. It lives at one stable, version-controlled
location in `compliance/ndaa-889/`:

```
Evidence (human):  https://github.com/scanopy/scanopy/blob/dev/compliance/ndaa-889/EVIDENCE.md
JSON:              https://github.com/scanopy/scanopy/raw/dev/compliance/ndaa-889/evidence.json
SBOMs:             https://github.com/scanopy/scanopy/raw/dev/compliance/ndaa-889/sbom-*.cdx.json
```

**Why `dev`, not `main`:** a repo ruleset ("PR to main only from dev") locks
`main` to PRs from `dev` via the `pr_dev_only` check, so no automation can push
there — `regenerate-db-enum-baseline` follows the same constraint. `dev` has no
such rule, so the workflow commits there directly; the evidence reaches `main`
automatically at the next `dev`→`main` release merge. `dev` is the always-current
copy (refreshes on demand); `main` reflects the last release. Cite whichever
suits the letter — `dev` for "current", `main` for "as-released".

`compliance/ndaa-889/` holds the current bundle only: `EVIDENCE.md` (result,
assessed commit, component count, tool + vendor-list versions/digests, per-image
status), `evidence.json`, the CycloneDX SBOMs, the exact policy files, and
`SHA256SUMS`. It is **overwritten each refresh** — latest only, no per-version
history (older states are reconstructable from the pinned commit + tool version).
Not a GitHub Release: Releases are for images/binaries.

**Refresh it** (before a deal, or on the monthly schedule): Actions tab →
"889 Evidence" → Run workflow. It runs `tools/889/889-evidence.sh` in CI and
commits the refreshed bundle to `compliance/ndaa-889/` on `dev`. It also
auto-runs after "Promote Release to Latest" so the evidence tracks production.

### Generating a bundle locally

`tools/889/889-evidence.sh` produces the same bundle on demand:

```sh
./tools/889/889-evidence.sh                      # source + the three :latest images
./tools/889/889-evidence.sh --tag v1.4.2         # a specific released tag
./tools/889/889-evidence.sh --no-images          # source tree only
```

It writes `889-evidence-<date>/` (gitignored): `EVIDENCE.md`, `evidence.json`,
`sbom-*.cdx.json`, `889-vendors.txt`, `889-allow.txt`, `summary.txt`,
`hits.jsonl` (only on a hit), `SHA256SUMS`. It **exits non-zero on a hit**, so a
clean bundle can never be produced for a tree that contains a covered-entity
component. A private image is recorded as `not-assessed` unless syft can pull it
(`docker login ghcr.io` first, or run in CI).

## Vendor list — seed methodology

`889-vendors.txt` entries are `pattern;source;date;note`. `pattern` is an ERE
matched case-insensitively against every component's identity fields (name,
group, publisher, author, supplier.name, purl, externalReference URLs). Seeded
from, in order of authority:

1. **Named covered entities** (`source=named-entity`) — the five entities named
   in NDAA Sec. 889(f)(3): Huawei, ZTE, Hytera, Hikvision, Dahua. Short/ambiguous
   tokens (ZTE, Nubia, Lorex) use portable word boundaries `(^|[^a-z0-9])…([^a-z0-9]|$)`
   to avoid matching substrings (e.g. "azteca" must not match "ZTE").
2. **Known subsidiaries / brand names** (`source=internal review`) — entities
   traceable to a covered entity: HiSilicon, HarmonyOS/OpenHarmony (Huawei);
   EZVIZ (Hikvision); Nubia (ZTE); Lorex (Dahua); plus legal-entity name variants.
3. **SAM.gov Exclusions, FAR 52.204-25** (`source=SAM.gov`) — federal excluded
   parties tagged to the 889 procurement clause. Seed pulled from the SAM.gov
   Exclusions extract: <https://sam.gov/data-services/Exclusions/Public>
   (Entity Management → Exclusions, filter cause/clause `FAR 52.204-25`).
4. **ASPI "Mapping China's Tech Giants"** (`source=ASPI`) — affiliate mapping used
   only where it adds patterns the lists above miss.
   <https://chinatechmap.aspi.org.au/>

Each row is date-stamped with its add/last-review date.

## Allowlist — false positives

`889-allow.txt` suppresses components a vendor pattern flags but that have been
reviewed and confirmed *not* to be covered-entity components. It only ever
subtracts documented exceptions; never add an entry to silence a real or
unexamined hit. Every entry cites why.

**Known false positive (seeded):** `@esbuild/openharmony-arm64` and
`@rollup/rollup-openharmony-arm64`. esbuild and rollup publish one optional
package per cross-compilation target; one target is Huawei's OpenHarmony OS.
These packages are published by the esbuild / rollup OSS projects (not Huawei)
and carry only that bundler's prebuilt binary. They match the `openharmony`
vendor pattern but are not covered-entity components, so they are allowlisted by
version-independent name. Disabling the allowlist (`--allow /dev/null`) makes the
current source SBOM fail on exactly these two — a convenient live demonstration
that the matcher catches pattern hits.

## Refresh mechanism

The covered-entity list changes over time (SAM.gov publishes updates; affiliates
are added). `.github/workflows/889-vendors-refresh.yml` runs **quarterly** and
opens (or updates) a tracking issue prompting a maintainer to re-pull SAM.gov
(FAR 52.204-25) and review ASPI, then update `889-vendors.txt` with new
patterns + date stamps.

**Why a review issue, not an automated fetch-and-PR (deferred):** a fully
automated job that fetches the SAM.gov dataset and opens a diff PR was
considered and deferred. The SAM.gov Entity Management API requires a registered
API key, and the Exclusions extract is a large structured download that needs
FAR-cause filtering and name-to-pattern translation — fragile to run unattended
on a quarterly cadence, and the matcher must stay offline (no run-time fetch).
A quarterly human review is the right cost/benefit; automating the fetch is a
separate future task if the manual cadence proves too slow.

**Feed check in the review issue.** The issue body includes the output of
`tools/889/889-candidates.sh`, which needs no API key. It downloads one dated
release of [us-restricted-parties](https://github.com/Mattbusel/us-restricted-parties)
(CC BY 4.0, rebuilt daily in public CI): `us_far_889.jsonl` (the five parties
from eCFR FAR 52.204-25) and `affiliates.senzing.jsonl` (companies those
parties own or control, from GLEIF Level 2 ownership links, each with its LEI
and proof path). Both files are checked against the SHA-256 in that release's
`manifest.json` / `affiliates.manifest.json`, and the release tag is printed so
the review records exactly which data it used. Every name that no pattern in
`889-vendors.txt` matches is listed with a ready-to-paste
`pattern;source;date;note` line. Affiliate rows are ownership evidence, not a
determination; a maintainer still decides what goes in the list. A feed failure
is noted in the issue and the manual checklist still applies. Run it locally:

```sh
./tools/889/889-candidates.sh                            # newest data-* release
./tools/889/889-candidates.sh --release data-2026-11-18  # a pinned release
./tools/889/test-889-candidates.sh                       # offline self-test (run by 889-check.yml)
```

To drop it, delete the two scripts and the feed-check step in
`889-vendors-refresh.yml`; nothing else depends on them.

## SBOM storage decision

**One home: `compliance/ndaa-889/` on `main`, latest only. The release job is a
gate, not a store.**

The 889 outputs split cleanly into two roles:

- **Gate** — the PR check (`889-check.yml`) and the release `supply-chain-889`
  job run the matcher and *fail* on a hit. They block bad code/releases; they do
  not store anything.
- **Evidence** — the published bundle a signed attestation letter links to. It
  lives in one place, `compliance/ndaa-889/`, refreshed by `889-evidence.yml`.

Keeping a single home was the deciding requirement: the customer cites **one**
link, not "the rolling one here and the per-release SBOMs there." So the release
job no longer attaches SBOMs to Releases (Releases are for images/binaries), and
the evidence is not pinned to a fake release tag.

We keep **only the latest** bundle (overwritten each refresh) rather than a
per-version history: old states are reconstructable from the pinned commit +
image digests + tool version recorded in `evidence.json`, and auditors want the
current state. This bounds git growth while still committing the actual SBOMs so
they are downloadable from the one link.

A CycloneDX SBOM here is non-sensitive: public OSS package names/versions/purls/
licenses, our already-public crate names, and syft-derived file paths — no
credentials, tokens, or customer data — so committing it to the public repo is
fine.

## Images scanned at release

The `supply-chain-889` job in `release.yml` runs after `create-manifests` and
scans the released source tree plus **both** published images:

- `ghcr.io/<repo>/server`
- `ghcr.io/<repo>/daemon`

Both are first-party artifacts shipped to users, so both are in scope.
The external `postgres` base image is pulled, not built by the release, and is
out of scope here; the Debian base packages inside our images *are* captured
when those images are scanned.

**Known tradeoff:** buildx pushes each image during its build job, so a scan
that runs after `create-manifests` cannot un-push a bad image. On a hit it fails
the release status and gates the separate, manual `promote_to_latest` workflow —
an effective gate even though the per-arch tags are already in GHCR. Moving the
scan strictly before push would require scanning each per-arch image inside its
matrix build job before `push: true`; deferred as a refinement (see Follow-ups).

The same three images are what `889-evidence.sh` assesses at `:latest` for the
published evidence — so the gate (per-release) and the evidence (current
production) cover the identical image set.

## Follow-ups (separate tasks)

- **Pre-push image scan** — scan each image in its matrix build job before
  `push: true` so a hit prevents the image reaching GHCR at all.
- **Automated SAM.gov fetch-and-PR** — replace the quarterly review issue with a
  job that pulls the FAR 52.204-25 extract and opens a diff PR (needs a SAM.gov
  API key + name-to-pattern translation).
- **Optional FOSSA license/CVE scanning** — orthogonal to 889 (license policy
  and vulnerabilities, not covered-entity identity); a separate effort if wanted.

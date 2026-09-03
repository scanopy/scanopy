# NDAA Section 889 Supply-Chain Review - Evidence

> Supporting evidence for a Section 889 compliance attestation. This records that
> an automated covered-entity review was performed over the SBOMs below.

**Result: PASS**

| Field | Value |
|-------|-------|
| Standard | NDAA FY2019 Section 889 (covered-entity components) |
| Generated (UTC) | 2026-09-03T19:16:34Z |
| Repository | `scanopy/scanopy` |
| Assessed commit | `1f12fb52c8e1b25719fb84869da45588c26cdab9` (1f12fb5-dirty) |
| Components assessed | 8138 |
| Prohibited-entity hits | 0 |
| Reviewed exceptions | 2 |
| SBOM generator | syft 1.45.1 |
| Matcher | `tools/889/check-889.sh` @ 1f12fb5 |
| Vendor list | `tools/889/889-vendors.txt` @ 1f12fb5 (sha256 `7f018a32dda6755f02f07a70cd76bc3e0a07c180a4dcf1bf7b23a118a663d611`) |

## Scope assessed

- Source tree at commit `1f12fb52c8e1b25719fb84869da45588c26cdab9`
- Image: `server` (`latest`)
- Image: `daemon` (`latest`)

## Statement

The CycloneDX SBOM(s) listed below were generated with syft and assessed
with `tools/889/check-889.sh` against the committed Section 889 covered-entity
vendor list. **No component originating with a covered entity (Huawei, ZTE,
Hytera, Hikvision, Dahua) or a known subsidiary/affiliate was found.**

2 reviewed exception(s) were suppressed as documented
false positives (see [889-allow.txt](889-allow.txt) and [summary.txt](summary.txt)).

## Files in this bundle

- [evidence.json](evidence.json) - machine-readable evidence record
- [summary.txt](summary.txt) - matcher human summary (counts + exceptions)
- [sbom-source.cdx.json](sbom-source.cdx.json) - CycloneDX SBOM
- [sbom-server.cdx.json](sbom-server.cdx.json) - CycloneDX SBOM
- [sbom-daemon.cdx.json](sbom-daemon.cdx.json) - CycloneDX SBOM
- [889-vendors.txt](889-vendors.txt), [889-allow.txt](889-allow.txt) - the exact policy used
- [SHA256SUMS](SHA256SUMS) - digests of every file above

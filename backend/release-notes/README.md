# Release notes

One `vX.Y.Z.md` per release, written by `prepare_release.yml` and reviewed in the release PR. The
server embeds these files: they fill the in-app release notes modal and the release email.

```markdown
---
version: 0.18.0
date: 2026-10-11
title: 'Short headline for the release'
email: true
modal: true
licensed_email: false
daemon_upgrade: true
---

Markdown body. Raw HTML is dropped when rendered.
```

`email: true` sends the release email to every user when a server starts on this version. The
email carries this release's notes only. A release with `email: false` is never emailed, and a later
emailed release doesn't include it. `email: false` shows the notes in the app only.

The email ends each section of the body (the text before each heading, and the last section) with an
Open Scanopy link to the app, and closes with an Open Scanopy button.

On Scanopy Cloud, owners of a current self-hosted licence get an upgrade version of the email in
place of the general one: the same notes, with each section's link and the closing button reading
Upgrade your server and pointing to the upgrade guide. Other
members of their org get neither, since their own server announces the release.
`licensed_email: true` sends that version to licence owners even when `email` is `false`. A file
without `licensed_email` sends it only with `email: true`. Self-hosted servers ignore it.

`modal: true` opens the release notes modal for each user after the upgrade, listing every release
they haven't dismissed. `modal: false` leaves it closed; the release's notes show the next time a
release opens it. A file without `modal` opens it.

`daemon_upgrade: true` asks users to upgrade their daemons. A daemon is outdated when it's older
than the newest release at or below the server's version with this flag, and the outdated warning
names that release. `daemon_upgrade: false` leaves daemons current against the last release that
asked. Keep it `true` when a release changes a daemon capability or the daemon wire format, since
those warnings fire whatever the flag says. A file without `daemon_upgrade` asks. Sunsets and
capability gates ignore it.

Prepare Release sets the four flags from its `announce_by_email`, `show_modal`,
`email_licensed_owners` and `daemon_upgrade` inputs.

Edit a file here before the release is tagged. A server shows the copy it was built with, so a
change after the tag reaches servers only with the next release.

## Images

A note can show a screenshot of a new feature. It appears in the release email, the in-app modal
and the website changelog.

```markdown
![The L2 topology view, showing each switch and the ports that link it to its neighbours](https://scanopy.net/changelog/v0.18.0/l2-topology.png)
```

1. Export the screenshot as PNG (or JPEG for a photo) at 1040px wide, twice the email's 520px
   content column. Keep it under 500 KB. Outlook can't show WebP.
2. Commit it to the website repo's `main` at `static/changelog/v0.18.0/l2-topology.png`, before
   merging the notes PR. Servers load it from scanopy.net as soon as they run the release, so
   it has to be live before the release is published. Publish Release fails while an image the
   notes use is missing there; commit it, then re-run Prepare Release.
3. Write alt text that says what the screenshot shows. Most mail clients block remote images
   until the reader allows them, and an air-gapped server can't load them at all, so the alt text
   is often all a reader sees.

CI fails a note whose image isn't under `https://scanopy.net/changelog/v<its version>/`, isn't
PNG or JPEG, or has no alt text.

## Terms updates

A change to Scanopy's terms is one `terms/YYYY-MM-DD.md`, named for the day it takes effect. Scanopy
Cloud emails notice of it to every verified user who signed up before that day, once each. The
release that ships the file sends it, and a daily sweep retries anyone the relay refused.
Self-hosted servers never send it.

Run `make new-terms-update` to create the file. It asks for the effective date and the documents,
prefilling each, and checks the date against the rules below. Set `EFFECTIVE` or `DOCUMENTS` to
skip a question. It also drafts each changed page in the website checkout
(`src/lib/legal/<page>/<effective>.svelte`, copied from the version in force) and opens an issue in
`scanopy/website`, assigned to you, to publish them. It needs the website checkout at `../website`
beside this repo (or `WEBSITE_DIR`) and a logged-in `gh`, and stops before writing anything without
them. Replace each `<describe the change>` it leaves in the terms file; a file that still holds one
fails CI.

```markdown
---
effective: 2026-12-01
date: 2026-10-20
documents: terms, privacy
---

- Terms of Service, section 4: we now email you before a significant change takes effect.
```

- `effective`: the day the update takes effect. It must match the file name.
- `date`: the day you wrote it. It can't be in the future.
- `documents`: any of `terms`, `privacy`, `refund`, `security`. The email links each one.
- The body says what changed, in plain terms. It goes into the email as written.

The notice goes out when the release carrying the file deploys, so cut the release the day you
create the file. The prefilled effective date is 30 days out; a notice sent with less than 30 days
to go logs an error. One the server hasn't sent by `effective` still goes out for 30 more days.

CI fails when an update takes effect less than 30 days or more than 180 days after `date`, or is
dated before an update that takes effect sooner.

The email links each changed document to `scanopy.net/<page>/<effective>`, which the website builds
from that draft. Edit the drafts and push them to website `main` before cutting the release:
Prepare Release fails while any terms file's pages are missing there. Until `effective`, the live
page links to the new version; on that day the website's `legal-effective-rebuild.yml` rebuilds the
site, and the live page switches to it.

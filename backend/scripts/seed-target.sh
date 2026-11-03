#!/bin/sh
# Starts a worktree's backend/target from the main checkout's third-party build output, as APFS
# clones: a 7 GB directory for about 1 MB of real disk, in about 4 seconds. Cargo then compiles only
# this worktree's own crates. macOS only; elsewhere the first build starts cold, as it would anyway.
set -eu

here=$(git rev-parse --show-toplevel)
main=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
source_dir="$main/backend/target"
target_dir="$here/backend/target"

if [ "$here" = "$main" ]; then
	echo "seed-target: run this in a worktree, not the main checkout." >&2
	exit 1
fi
if [ -e "$target_dir" ]; then
	echo "seed-target: $target_dir already exists, leaving it alone."
	exit 0
fi
if [ ! -d "$source_dir/debug/deps" ]; then
	echo "seed-target: nothing to clone at $source_dir, the first build starts cold."
	exit 0
fi
# A build in the main checkout writes these files as we read them.
if lsof "$source_dir/debug/.cargo-lock" >/dev/null 2>&1; then
	echo "seed-target: a build is running in $main, the first build starts cold."
	exit 0
fi

mkdir -p "$target_dir/debug/deps"
cp -cp "$source_dir/.rustc_info.json" "$source_dir/CACHEDIR.TAG" "$target_dir/"
cp -cpR "$source_dir/debug/build" "$source_dir/debug/.fingerprint" "$target_dir/debug/"
# This repo's own crates are keyed on the worktree's path, so their output is no use here.
find "$source_dir/debug/deps" -maxdepth 1 -type f ! -name '*.rcgu.o' \
	! -name '*scanopy*' ! -name 'server-*' ! -name 'daemon-*' ! -name 'migrate-*' \
	! -name 'license-*' ! -name 'generate_*' -print0 |
	xargs -0 -J % cp -cp % "$target_dir/debug/deps/"
echo "seed-target: cloned third-party crates from $source_dir"

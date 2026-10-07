#!/usr/bin/env bash
# Local smoke test for release.yml's migration-branch-test pipeline. Mirrors the
# workflow's Neon-branching + migration-apply flow so you can verify real behavior
# (branch forks from prod, migrations apply correctly, cleanup runs, bad migrations
# fail cleanly) from your laptop before the first live release exercises it.
#
# Requirements:
#   - NEON_API_KEY     — same token the Neon GitHub integration uses
#   - NEON_PROJECT_ID  — same project ID the workflow uses
#   - cargo            — builds and runs the bin/migrate runner, the one release.yml uses
#   - pg_dump          — from postgresql-client (or `brew install libpq`)
#   - neonctl          — auto-invoked via `npx -y neonctl`
#
# Usage:
#   ./backend/scripts/test-migration-branch.sh <stage>
#
# Stages (run in order):
#   create-only  — create + delete a Neon branch. Cheapest proof that the token,
#                  project ID, and parent-branch default all work.
#   apply-noop   — full pipeline against prod. bin/migrate is a no-op
#                  because all current migrations are already applied on prod;
#                  diff is empty. Proves end-to-end happy path.
#   apply-good   — injects a harmless test migration at runtime, applies it,
#                  shows the schema diff (new table), then cleans up (branch
#                  delete drops the table with it; file is removed from disk).
#   apply-bad    — injects a deliberately broken migration. The runner must fail.
#                  Confirms the failure path and cleanup still run.

set -euo pipefail

STAGE="${1:-}"
case "$STAGE" in
    create-only|apply-noop|apply-good|apply-bad) ;;
    *)
        echo "Usage: $0 <create-only|apply-noop|apply-good|apply-bad>" >&2
        exit 1
        ;;
esac

: "${NEON_API_KEY:?NEON_API_KEY must be set (export NEON_API_KEY=...)}"
: "${NEON_PROJECT_ID:?NEON_PROJECT_ID must be set (export NEON_PROJECT_ID=...)}"
export NEON_API_KEY

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BRANCH_NAME="local-smoke/$STAGE-$(date +%s)"
NEONCTL=(npx -y neonctl)

INJECTED_GOOD="$BACKEND_DIR/migrations/99999999999999_smoke_good.sql"
INJECTED_BAD="$BACKEND_DIR/migrations/99999999999999_smoke_bad.sql"
BEFORE_DUMP="$(mktemp)"
AFTER_DUMP="$(mktemp)"

cleanup() {
    local rc=$?
    echo
    echo "=== Cleanup ==="
    rm -f "$INJECTED_GOOD" "$INJECTED_BAD" "$BEFORE_DUMP" "$AFTER_DUMP"
    echo "  removed any injected test migration files + temp dumps"
    if "${NEONCTL[@]}" branches get "$BRANCH_NAME" --project-id "$NEON_PROJECT_ID" >/dev/null 2>&1; then
        echo "  deleting Neon branch $BRANCH_NAME ..."
        "${NEONCTL[@]}" branches delete "$BRANCH_NAME" --project-id "$NEON_PROJECT_ID" >/dev/null 2>&1 \
            && echo "  ✓ branch deleted" \
            || echo "  ✗ branch delete failed — clean up manually in the Neon console"
    fi
    exit "$rc"
}
trap cleanup EXIT INT TERM

echo "=== Project branches (confirm parent/default is PROD before continuing) ==="
"${NEONCTL[@]}" branches list --project-id "$NEON_PROJECT_ID"
echo

# Cross-platform "+1 hour" for --expiration-date.
if EXPIRES_AT=$(date -u -v+1H +'%Y-%m-%dT%H:%M:%SZ' 2>/dev/null); then :; else
    EXPIRES_AT=$(date -u -d '+1 hour' +'%Y-%m-%dT%H:%M:%SZ')
fi

echo "=== Creating Neon branch: $BRANCH_NAME ==="
"${NEONCTL[@]}" branches create \
    --project-id "$NEON_PROJECT_ID" \
    --name "$BRANCH_NAME" \
    --expiration-date "$EXPIRES_AT"

echo
echo "=== Parent of new branch (verify this matches your PROD branch id from the list above) ==="
"${NEONCTL[@]}" branches get "$BRANCH_NAME" --project-id "$NEON_PROJECT_ID" --output json \
    | grep -E '"(name|id|parent_id|parent_lsn)"' \
    | sed 's/^/  /'

DATABASE_URL="$("${NEONCTL[@]}" connection-string "$BRANCH_NAME" --project-id "$NEON_PROJECT_ID")"
export DATABASE_URL

if [ "$STAGE" = "create-only" ]; then
    echo
    echo "=== Stage 'create-only' complete — cleanup will delete the branch. ==="
    exit 0
fi

echo
echo "=== Snapshotting schema before migrations ==="
pg_dump --schema-only --no-owner --no-privileges "$DATABASE_URL" \
    | grep -Ev '^\\(un)?restrict' > "$BEFORE_DUMP"
echo "  ($(wc -l <"$BEFORE_DUMP" | tr -d ' ') lines)"

case "$STAGE" in
    apply-good)
        echo
        echo "=== Injecting good test migration ==="
        cat > "$INJECTED_GOOD" <<'SQL'
-- Smoke-test migration injected by test-migration-branch.sh. Not committed.
SET lock_timeout = '5s';
CREATE TABLE migration_branch_smoke_test (id integer PRIMARY KEY);
SQL
        ;;
    apply-bad)
        echo
        echo "=== Injecting bad test migration ==="
        cat > "$INJECTED_BAD" <<'SQL'
-- Smoke-test migration injected by test-migration-branch.sh. Not committed. Expected to fail.
CREATE TABLE migration_branch_smoke_test (id NONEXISTENT_TYPE);
SQL
        ;;
esac

echo
echo "=== Running bin/migrate ==="
set +e
# The same runner release.yml uses. sqlx-cli can't apply this repo's `-- no-transaction`
# migrations. --migrations-dir reads from disk so an injected test migration is picked up.
(cd "$BACKEND_DIR" && cargo run --bin migrate -- --database-url "$DATABASE_URL" --migrations-dir migrations)
MIGRATE_RC=$?
set -e

if [ "$STAGE" = "apply-bad" ]; then
    if [ "$MIGRATE_RC" -eq 0 ]; then
        echo "FAIL: bad migration unexpectedly succeeded." >&2
        exit 1
    fi
    echo "✓ Bad migration failed with exit $MIGRATE_RC, as expected."
else
    if [ "$MIGRATE_RC" -ne 0 ]; then
        echo "FAIL: bin/migrate returned $MIGRATE_RC (expected 0 for stage $STAGE)." >&2
        exit "$MIGRATE_RC"
    fi
    echo "✓ bin/migrate succeeded."
fi

echo
echo "=== Snapshotting schema after migrations ==="
pg_dump --schema-only --no-owner --no-privileges "$DATABASE_URL" \
    | grep -Ev '^\\(un)?restrict' > "$AFTER_DUMP"
echo "  ($(wc -l <"$AFTER_DUMP" | tr -d ' ') lines)"

echo
echo "=== Schema diff ==="
if diff -q "$BEFORE_DUMP" "$AFTER_DUMP" >/dev/null; then
    echo "  (empty — no schema changes were applied)"
else
    diff -u "$BEFORE_DUMP" "$AFTER_DUMP"
fi

echo
echo "=== Stage '$STAGE' complete. ==="

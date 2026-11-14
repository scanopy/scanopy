#!/usr/bin/env bash
# Backward-compat container harness: pulls the currently-deployed server image,
# boots it against an already-migrated database, and polls liveness + readiness
# to prove the old binary can still speak the new schema during a deploy-window
# coexistence period.
#
# Called from:
#   - .github/workflows/server-ci.yml (PR-time, gated on migration file changes)
#   - .github/workflows/release.yml migration-branch-test (release-time, final gate)
#
# Local dry-run:
#   bash backend/scripts/compat-check.sh "$DATABASE_URL" "ghcr.io/scanopy/scanopy/server:v0.14.0"
#
# With --new-release it checks the image being released instead (release.yml neon-smoke-test):
# the image must exist, and once ready the container must stay up for WATCH_SECONDS with no
# panic or database error in its logs. v0.20.0 passed a boot check and then failed every query
# once its background tasks ran (2026-11-13).
#
# Exit codes:
#   0 — all checks passed, OR prior image unavailable (first-release fallback),
#       OR readiness returned 404 (prior image predates the readiness endpoint).
#   1 — liveness failed, readiness never 200, container exited, a --new-release watch saw a
#       panic or database error, or other hard failure.
#
# When the script prints a line beginning with COMPAT_STATUS=, the caller is
# expected to tee it into $GITHUB_STEP_SUMMARY so the outcome is visible in the
# GitHub Actions UI.

set -euo pipefail

DB_URL="${1:-}"
IMAGE_REF="${2:-}"
NEW_RELEASE=false
[ "${3:-}" = "--new-release" ] && NEW_RELEASE=true

if [ -z "$DB_URL" ] || [ -z "$IMAGE_REF" ]; then
    echo "Usage: $0 <DATABASE_URL> <IMAGE_REF> [--new-release]" >&2
    exit 2
fi

CONTAINER_NAME="scanopy-compat-$$"
HOST_PORT="${COMPAT_HOST_PORT:-60072}"
LIVENESS_BUDGET="${COMPAT_LIVENESS_BUDGET:-30}"
READINESS_BUDGET="${COMPAT_READINESS_BUDGET:-30}"
POLL_INTERVAL="${COMPAT_POLL_INTERVAL:-2}"
WATCH_SECONDS="${COMPAT_WATCH_SECONDS:-60}"

cleanup() {
    local rc=$?
    echo
    echo "=== Cleanup ==="
    if docker inspect "$CONTAINER_NAME" >/dev/null 2>&1; then
        if [ "$rc" -ne 0 ]; then
            echo "--- Container logs (last 200 lines) ---"
            docker logs "$CONTAINER_NAME" 2>&1 | tail -200 || true
            echo "--- End container logs ---"
        fi
        docker stop "$CONTAINER_NAME" >/dev/null 2>&1 || true
        docker rm "$CONTAINER_NAME" >/dev/null 2>&1 || true
        echo "  container $CONTAINER_NAME stopped + removed"
    fi
    exit "$rc"
}
trap cleanup EXIT INT TERM

echo "=== Pulling image: $IMAGE_REF ==="
if ! docker pull "$IMAGE_REF" 2>&1; then
    if $NEW_RELEASE; then
        echo "COMPAT_STATUS=FAIL: release image $IMAGE_REF not available in registry."
        trap - EXIT
        exit 1
    fi
    echo
    echo "COMPAT_STATUS=SKIPPED: prior image $IMAGE_REF not available in registry (first release, or retention-pruned). Backward-compat check skipped."
    # Disarm the trap — we have no container to clean up and don't want to
    # re-enter cleanup with a zero rc that might surprise callers.
    trap - EXIT
    exit 0
fi

echo
echo "=== Starting container against migrated database ==="
# The server reads its DB URL from a SCANOPY_-prefixed env var (figment
# Env::prefixed("SCANOPY_") in server/config.rs). A bare DATABASE_URL is
# ignored, so the server fell back to its localhost default and the pool
# acquire timed out — pass SCANOPY_DATABASE_URL.
docker run -d \
    --name "$CONTAINER_NAME" \
    -e SCANOPY_DATABASE_URL="$DB_URL" \
    -p "${HOST_PORT}:60072" \
    "$IMAGE_REF" >/dev/null

echo "  container: $CONTAINER_NAME"
echo "  port: $HOST_PORT"

poll_status() {
    # Prints the HTTP status code, or "000" on connection failure.
    curl --silent --output /dev/null --write-out '%{http_code}' \
        --max-time 3 \
        "http://localhost:${HOST_PORT}$1" 2>/dev/null || echo "000"
}

container_running() {
    [ "$(docker inspect --format='{{.State.Running}}' "$CONTAINER_NAME" 2>/dev/null || echo false)" = "true" ]
}

echo
echo "=== Polling /api/health (liveness, budget ${LIVENESS_BUDGET}s) ==="
elapsed=0
while [ "$elapsed" -lt "$LIVENESS_BUDGET" ]; do
    if ! container_running; then
        echo "COMPAT_STATUS=FAIL: container exited during startup — see logs above."
        exit 1
    fi
    status=$(poll_status /api/health)
    if [ "$status" = "200" ]; then
        echo "  ✓ /api/health 200 after ${elapsed}s"
        break
    fi
    sleep "$POLL_INTERVAL"
    elapsed=$((elapsed + POLL_INTERVAL))
done
if [ "$status" != "200" ]; then
    echo "COMPAT_STATUS=FAIL: /api/health did not return 200 within ${LIVENESS_BUDGET}s (last status: $status)."
    exit 1
fi

echo
echo "=== Polling /api/health/ready (readiness, budget ${READINESS_BUDGET}s) ==="
elapsed=0
while [ "$elapsed" -lt "$READINESS_BUDGET" ]; do
    if ! container_running; then
        echo "COMPAT_STATUS=FAIL: container exited during readiness polling — see logs above."
        exit 1
    fi
    status=$(poll_status /api/health/ready)
    case "$status" in
        200)
            echo "  ✓ /api/health/ready 200 after ${elapsed}s"
            break
            ;;
        404)
            if $NEW_RELEASE; then
                echo "COMPAT_STATUS=FAIL: release image has no /api/health/ready."
                exit 1
            fi
            echo "  ℹ /api/health/ready 404 — prior image predates the readiness endpoint."
            echo
            echo "COMPAT_STATUS=PARTIAL: prior image has no readiness endpoint (first release after backward-compat check landed). Downgraded to process-stayed-up check; /api/health succeeded. Subsequent releases will exercise the full DB probe."
            exit 0
            ;;
    esac
    # 503 means starting (background tasks not yet up and probed) or a failed probe; keep
    # polling until the budget runs out.
    sleep "$POLL_INTERVAL"
    elapsed=$((elapsed + POLL_INTERVAL))
done
if [ "$status" != "200" ]; then
    echo "COMPAT_STATUS=FAIL: /api/health/ready did not return 200 within ${READINESS_BUDGET}s (last status: $status). The binary cannot execute SQL against the migrated schema. See logs above."
    exit 1
fi

if ! $NEW_RELEASE; then
    echo
    echo "COMPAT_STATUS=PASS: previous release binary can execute SQL against migrated schema."
    exit 0
fi

# Background tasks query on their own schedule after startup. Watch them run.
echo
echo "=== Watching the running server (${WATCH_SECONDS}s) ==="
elapsed=0
while [ "$elapsed" -lt "$WATCH_SECONDS" ]; do
    if ! container_running; then
        echo "COMPAT_STATUS=FAIL: release container exited ${elapsed}s after becoming ready. See logs above."
        exit 1
    fi
    sleep "$POLL_INTERVAL"
    elapsed=$((elapsed + POLL_INTERVAL))
done
# A panic aborts the process, which the loop above catches; a database error a task logs and
# survives shows up here.
if docker logs "$CONTAINER_NAME" 2>&1 | grep -E "panicked at|error returned from database|prepared statement" >/tmp/compat-errors.log; then
    echo "--- Errors in the release container's log ---"
    head -50 /tmp/compat-errors.log
    echo "COMPAT_STATUS=FAIL: the release binary logged database errors while its background tasks ran."
    exit 1
fi
echo
echo "COMPAT_STATUS=PASS: release binary served readiness and ran ${WATCH_SECONDS}s against the migrated schema with no panic or database error."
exit 0

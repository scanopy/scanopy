#!/usr/bin/env bash
# Lint migrations dated 20260501 onward with squawk. Earlier migrations predate
# the Scanopy safety guidelines and are intentionally excluded — do not expand
# this cutoff without updating CLAUDE.md.
set -euo pipefail

CUTOFF="20260501"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATIONS_DIR="$SCRIPT_DIR/../migrations"
CONFIG_PATH="$SCRIPT_DIR/../../.squawk.toml"

if [ ! -d "$MIGRATIONS_DIR" ]; then
    echo "lint-migrations: migrations directory not found: $MIGRATIONS_DIR" >&2
    exit 1
fi

in_tx_files=()
no_tx_files=()
for f in "$MIGRATIONS_DIR"/*.sql; do
    [ -e "$f" ] || continue
    name="$(basename "$f")"
    prefix="${name:0:8}"
    case "$prefix" in
        [0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]) ;;
        *) continue ;;
    esac
    if [[ "$prefix" > "$CUTOFF" ]] || [[ "$prefix" == "$CUTOFF" ]]; then
        # sqlx skips the per-migration transaction wrapper for files that start
        # with a `-- no-transaction` comment. Mirror that for squawk so its
        # ban-concurrent-index-creation-in-transaction rule doesn't false-positive
        # on CONCURRENTLY-using migrations.
        if [ "$(head -n 1 "$f")" = "-- no-transaction" ]; then
            no_tx_files+=("$f")
        else
            in_tx_files+=("$f")
        fi
    fi
done

if [ ${#in_tx_files[@]} -eq 0 ] && [ ${#no_tx_files[@]} -eq 0 ]; then
    echo "No post-$CUTOFF migrations to lint."
    exit 0
fi

if ! command -v squawk >/dev/null 2>&1; then
    echo "lint-migrations: squawk not found on PATH. Install with: npm install -g squawk-cli" >&2
    exit 1
fi

status=0
# Per-file rule exclusions for migrations whose unsafety is intentional and
# acknowledged in the migration's own header comment. Add to the relevant
# array as needed. squawk's `--exclude-path` is path-pattern based; we exclude
# entire rules per file by partitioning into separate squawk invocations.
DOWNTIME_FILES=(
    "$MIGRATIONS_DIR/20260502120004_drop_legacy_topology_columns.sql"
    # Contract half of the plan_limit_notifications -> notifications rename.
    # v0.17.6-v0.17.8 containers dual-write the dropped column, so this is NOT a
    # no-reader drop -- it is only safe because v0.17.9 ships as a downtime deploy.
    "$MIGRATIONS_DIR/20260803120000_drop_organizations_plan_limit_notifications.sql"
    # GH #701 multi-neighbour schema: drops the 12 legacy neighbour/LLDP/CDP
    # columns on `interfaces` in the same migration that creates their
    # replacement tables. No expand/contract -- ships as a downtime deploy.
    "$MIGRATIONS_DIR/20260903120001_interface_neighbor_tables.sql"
)
FK_BACKFILL_FILES=("$MIGRATIONS_DIR/20260502120001_add_snapshot_id_fks.sql")
# Columns dropped that have NO live code readers at the currently-deployed release,
# so the drop is safe under a rolling deploy (contract already paid). Header comment
# in each migration documents why. Suppress ban-drop-column.
NO_READER_DROP_FILES=(
    "$MIGRATIONS_DIR/20260703120001_drop_discovery_pending_credential_ids.sql"
    "$MIGRATIONS_DIR/20260706120000_drop_credentials_target_ips_and_daemons_capabilities.sql"
)
# NOT NULL dropped so a column can record "never read" instead of an invented value.
# The released container reads these with `row.get::<i32>`, which panics on NULL, so the
# release ships as a downtime deploy and the two containers never coexist. Header comment
# in each migration documents this. Suppress ban-drop-not-null.
DROP_NOT_NULL_FILES=(
    "$MIGRATIONS_DIR/20260827130000_interfaces_unknown_port_facts.sql"
    "$MIGRATIONS_DIR/20260903120000_interfaces_if_descr_optional.sql"
)

# Filter file lists.
in_tx_main=()
for f in "${in_tx_files[@]}"; do
    skip=0
    for d in "${DOWNTIME_FILES[@]}" "${FK_BACKFILL_FILES[@]}" "${NO_READER_DROP_FILES[@]}" "${DROP_NOT_NULL_FILES[@]}"; do
        if [ "$f" = "$d" ]; then skip=1; break; fi
    done
    if [ "$skip" = "0" ]; then in_tx_main+=("$f"); fi
done

if [ ${#in_tx_main[@]} -gt 0 ]; then
    squawk --config "$CONFIG_PATH" "${in_tx_main[@]}" || status=$?
fi
if [ ${#no_tx_files[@]} -gt 0 ]; then
    squawk --config "$CONFIG_PATH" --no-assume-in-transaction "${no_tx_files[@]}" || status=$?
fi

# Downtime migration: drops legacy columns. Header comment in the migration
# documents the deploy mode (stop-migrate-start). Suppress ban-drop-column.
for f in "${DOWNTIME_FILES[@]}"; do
    if [ -e "$f" ]; then
        squawk --config "$CONFIG_PATH" --exclude=ban-drop-column "$f" || status=$?
    fi
done

# Snapshot FK backfill: adds NULLABLE FK columns to the empty `snapshots`
# table. The validation scan is fast because every existing row has
# snapshot_id IS NULL. lock_timeout = '5s' prevents long-held locks.
for f in "${FK_BACKFILL_FILES[@]}"; do
    if [ -e "$f" ]; then
        squawk --config "$CONFIG_PATH" --exclude=adding-foreign-key-constraint "$f" || status=$?
    fi
done

# No-reader column drops: safe under rolling deploy (no deployed container reads
# the column). Header comment in each migration documents why. Suppress
# ban-drop-column.
for f in "${NO_READER_DROP_FILES[@]}"; do
    if [ -e "$f" ]; then
        squawk --config "$CONFIG_PATH" --exclude=ban-drop-column "$f" || status=$?
    fi
done

# Nullability relaxations: see DROP_NOT_NULL_FILES above.
for f in "${DROP_NOT_NULL_FILES[@]}"; do
    if [ -e "$f" ]; then
        squawk --config "$CONFIG_PATH" --exclude=ban-drop-not-null "$f" || status=$?
    fi
done

exit "$status"

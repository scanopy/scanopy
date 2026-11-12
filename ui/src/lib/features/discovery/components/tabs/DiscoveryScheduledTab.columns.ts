import type { Snippet } from 'svelte';
import daemonVersionGates from '#lib/data/daemon-version-gates.json';
import { discoveryTypes } from '#lib/shared/stores/metadata.ts';
import type { Discovery } from '../../types/base';
import {
	discoveryFields,
	formatScheduleDisplay,
	type DiscoveryFieldEntry,
	type DiscoveryConfigOrderField,
	type DiscoveryListParam,
	type DiscoverySharedOrderField
} from '../../queries';
import type { DisplayConfig } from '#lib/shared/components/data/types.ts';
import { defineRegistryFields } from '#lib/shared/entity-fields/table.ts';
import type { EntityLookup } from '#lib/shared/entity-fields/index.ts';
import { fieldValueOptions, type FieldValueCount } from '#lib/shared/api/field-values.ts';
import type { DiscoveryUpdatePayload } from '../../types/api';
import type { Daemon } from '#lib/features/daemons/types/base.ts';
import type { Site } from '#lib/features/sites/types.ts';
import type { Tag } from '#lib/features/tags/types/base.ts';
import {
	common_legacy,
	common_manual,
	discovery_schedulePausedFreePlan,
	discovery_schedulePausedLapsed,
	common_tags,
	discovery_legacyTagTitle,
	discovery_runType
} from '#lib/paraglide/messages.js';

/**
 * How the shared discovery fields sit on this tab. `discovery_type` is the same
 * on every scheduled scan, so it is off by default and serves mostly as a filter
 * and group axis. The audit dates stay available from the column menu.
 */
const SHARED_FIELD_DISPLAY: Partial<Record<DiscoverySharedOrderField, DisplayConfig<Discovery>>> = {
	name: { order: 0 },
	site_id: { order: 2 },
	daemon_id: { order: 3 },
	discovery_type: { hiddenByDefault: true },
	created_at: { hiddenByDefault: true },
	updated_at: { hiddenByDefault: true }
};

function withSharedDisplay(
	shared: Record<DiscoverySharedOrderField, DiscoveryFieldEntry>
): Record<DiscoverySharedOrderField, DiscoveryFieldEntry> {
	const fields = { ...shared };
	for (const [key, overrides] of Object.entries(SHARED_FIELD_DISPLAY)) {
		const orderField = key as DiscoverySharedOrderField;
		fields[orderField] = {
			...fields[orderField],
			display: { ...fields[orderField].display, ...overrides }
		};
	}
	return fields;
}

/** What the scheduled scan columns read from the tab, each as a getter or function so values stay live. */
export interface DiscoveryScheduledColumnsContext {
	lookup: () => EntityLookup;
	daemons: () => Daemon[];
	sites: () => Site[];
	tags: () => Tag[];
	scheduleLapsed: () => boolean;
	schedulePaused: () => boolean;
	getActiveSession: (discovery: Discovery) => DiscoveryUpdatePayload | null;
	progressCell: () => Snippet<[Discovery, string]>;
	daemonValues: () => FieldValueCount[] | undefined;
	siteValues: () => FieldValueCount[] | undefined;
	typeValues: () => FieldValueCount[] | undefined;
	runTypeValues: () => FieldValueCount[] | undefined;
}

/** Every DiscoveryConfigOrderField value is listed; the server sorts, groups and filters each. */
export function discoveryScheduledColumns(ctx: DiscoveryScheduledColumnsContext) {
	const shared = withSharedDisplay(discoveryFields(ctx.daemons(), ctx.sites()));
	const orderable: Record<DiscoveryConfigOrderField, DiscoveryFieldEntry> = {
		...shared,
		daemon_id: {
			...shared.daemon_id,
			serverFiltered: true,
			// The daemons some scan runs on.
			filterOptions: fieldValueOptions(ctx.daemonValues()),
			// Displayed as a name, but grouped by id on the server.
			getGroupValue: (item) => item.daemon_id
		},
		site_id: {
			...shared.site_id,
			serverFiltered: true,
			// The sites some scan covers.
			filterOptions: fieldValueOptions(ctx.siteValues()),
			getGroupValue: (item) => item.site_id
		},
		discovery_type: {
			...shared.discovery_type,
			serverFiltered: true,
			// The types some scan has.
			filterOptions: fieldValueOptions(ctx.typeValues()),
			getGroupValue: (item) => item.discovery_type.type
		},
		scan_count: {
			type: 'string',
			// A count: it sorts, but every value would be its own group.
			groupable: false,
			getValue: (item) => String(item.scan_count ?? 0),
			display: { hiddenByDefault: true, align: 'right' }
		},
		force_full_scan: {
			type: 'boolean',
			filterable: true,
			serverFiltered: true,
			groupable: true,
			getGroupValue: (item) => String(item.force_full_scan ?? false),
			getValue: (item) => item.force_full_scan ?? false,
			display: { hiddenByDefault: true }
		},
		legacy: {
			type: 'boolean',
			filterable: true,
			serverFiltered: true,
			groupable: true,
			// Legacy-ness comes from the backend's own `is_legacy`, which the server also orders and
			// filters on — a `!== 'Unified'` check flagged Rescan, which is new rather than frozen.
			getGroupValue: (item) =>
				String(discoveryTypes.getMetadata(item.discovery_type.type).is_legacy ?? false),
			getValue: (item) => discoveryTypes.getMetadata(item.discovery_type.type).is_legacy ?? false,
			display: {
				getItems: (item) =>
					discoveryTypes.getMetadata(item.discovery_type.type).is_legacy
						? [
								{
									id: 'legacy',
									label: common_legacy(),
									color: 'Yellow',
									title: discovery_legacyTagTitle({
										version: daemonVersionGates.last_without_unified_discovery
									})
								}
							]
						: []
			}
		},
		run_type: {
			label: discovery_runType(),
			type: 'string',
			filterable: true,
			serverFiltered: true,
			queryParam: 'run_types',
			// The ways some scan runs.
			filterOptions: fieldValueOptions(ctx.runTypeValues()),
			getFilterValue: (item) => item.run_type.type,
			getGroupValue: (item) => item.run_type.type,
			display: { order: 1 }
		},
		last_run: {
			// A date column so it sorts by time. The ISO string rather than a Date, so the cell
			// renders it as recent activity (a Date renders as a compact date).
			type: 'date',
			groupable: false,
			getValue: (item) =>
				item.run_type.type !== 'Historical' && item.run_type.last_run
					? item.run_type.last_run
					: null,
			display: { recency: true }
		}
	};

	return defineRegistryFields<Discovery, DiscoveryConfigOrderField, DiscoveryListParam>(
		'Discovery',
		ctx.lookup,
		orderable,
		[
			{
				key: 'schedule',
				type: 'string',
				getValue: (item) =>
					item.run_type.type !== 'Scheduled'
						? common_manual()
						: ctx.scheduleLapsed()
							? discovery_schedulePausedLapsed()
							: ctx.schedulePaused()
								? discovery_schedulePausedFreePlan()
								: formatScheduleDisplay(item.run_type.cron_schedule, item.run_type.timezone),
				display: { hiddenByDefault: true }
			},
			{
				key: 'progress',
				// Progress belongs to the run, not the record, so there is nothing to
				// sort, filter or group by — but it is still a field, so a running scan
				// shows its tracker in the table.
				type: 'string',
				getValue: (item) => ctx.getActiveSession(item)?.phase ?? '',
				// After the tags column, immediately before the row actions: it is the
				// row's live state rather than one of its attributes.
				display: { trailing: true, cell: ctx.progressCell() }
			},
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				filterable: true,
				getValue: (entity) =>
					entity.tags
						.map((id) => ctx.tags().find((t) => t.id === id)?.name)
						.filter((name): name is string => !!name)
			}
		]
	);
}

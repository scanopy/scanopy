import type { Snippet } from 'svelte';
import { discoveryPhases, discoveryTypes } from '$lib/shared/stores/metadata';
import type { Discovery } from '../../types/base';
import { formatDuration } from '$lib/shared/utils/formatting';
import { defineRegistryFields } from '$lib/shared/entity-fields/table';
import type { EntityLookup } from '$lib/shared/entity-fields';
import { fieldValueOptions, type FieldValueCount } from '$lib/shared/api/field-values';
import type { operations } from '$lib/api/schema';
import { daemonItems } from '$lib/features/daemons/columns';
import { siteItems } from '$lib/features/sites/columns';
import { tagItems, tagNames } from '$lib/features/tags/columns';
import { toColor } from '$lib/shared/utils/styling';
import type { OutcomeTag } from '../../utils/outcome';
import { isFullScan } from '../../utils/fullScan';
import type { Daemon } from '$lib/features/daemons/types/base';
import type { Site } from '$lib/features/sites/types';
import type { Tag, TagGroup } from '$lib/features/tags/types/base';
import {
	common_daemon,
	common_full,
	common_light,
	common_tags,
	common_site,
	common_type,
	common_unknown,
	common_unknownEntity,
	common_unknownSite
} from '$lib/paraglide/messages';

import type { DiscoveryRunOrderField } from '../../queries';
/** The discovery list endpoint's query params, which the filters are sent and linked as. */
type DiscoveryListParam = keyof NonNullable<operations['list_discoveries']['parameters']['query']>;
type DiscoveryResults = Extract<Discovery['run_type'], { type: 'Historical' }>['results'];

/** What the run history columns read from the tab, each as a getter or function so values stay live. */
export interface DiscoveryHistoryColumnsContext {
	lookup: () => EntityLookup;
	daemons: () => Daemon[];
	sites: () => Site[];
	tags: () => Tag[];
	tagGroups: () => TagGroup[] | undefined;
	daemonValues: () => FieldValueCount[] | undefined;
	siteValues: () => FieldValueCount[] | undefined;
	typeValues: () => FieldValueCount[] | undefined;
	phaseValues: () => FieldValueCount[] | undefined;
	resultsOf: (discovery: Discovery) => DiscoveryResults | null;
	outcomeTag: (discovery: Discovery) => OutcomeTag | null;
	warningsOf: (discovery: Discovery) => NonNullable<DiscoveryResults['warnings']>;
	startedAtCell: () => Snippet<[Discovery, string]>;
	finishedAtCell: () => Snippet<[Discovery, string]>;
}

export function discoveryHistoryColumns(ctx: DiscoveryHistoryColumnsContext) {
	return defineRegistryFields<Discovery, DiscoveryRunOrderField, DiscoveryListParam>(
		'Discovery',
		ctx.lookup,
		{
			// Runs of one scan share its name, so grouping by it gathers a scan's history.
			name: {
				searchable: true,
				display: { order: 0 }
			},
			daemon_id: {
				label: common_daemon(),
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'daemon_ids',
				// The daemons some run was made by.
				filterOptions: fieldValueOptions(ctx.daemonValues()),
				filterLabel: (id) =>
					ctx.daemons().find((d) => d.id === id)?.name ??
					common_unknownEntity({ entity: common_daemon() }),
				groupable: true,
				// Displayed as a name, but grouped by id on the server.
				getGroupValue: (item) => item.daemon_id,
				getValue: (item) =>
					ctx.daemons().find((d) => d.id === item.daemon_id)?.name ??
					common_unknownEntity({ entity: common_daemon() }),
				display: { order: 8, getItems: (item) => daemonItems(item.daemon_id, ctx.daemons()) }
			},
			site_id: {
				label: common_site(),
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				// The sites some run scanned.
				filterOptions: fieldValueOptions(ctx.siteValues()),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? common_unknownSite(),
				groupable: true,
				getGroupValue: (item) => item.site_id,
				getValue: (item) =>
					ctx.sites().find((n) => n.id === item.site_id)?.name ?? common_unknownSite(),
				display: { order: 7, getItems: (item) => siteItems(item.site_id, ctx.sites()) }
			},
			discovery_type: {
				label: common_type(),
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'discovery_types',
				// The types some run had.
				filterOptions: fieldValueOptions(ctx.typeValues()),
				filterLabel: (id) => discoveryTypes.getName(id),
				groupable: true,
				// The server groups on the raw discriminant; the column renders its name.
				getGroupValue: (item) => item.discovery_type.type,
				getValue: (item) => discoveryTypes.getName(item.discovery_type.type),
				display: { hiddenByDefault: true }
			},
			created_at: { type: 'date', display: { hiddenByDefault: true } },
			updated_at: { type: 'date', display: { hiddenByDefault: true } },
			// The run's outcome. Sorted, grouped and
			// filtered by the terminal phase; the cell shows the outcome tag, whose hover carries
			// the reason the run ended.
			phase: {
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'phases',
				// The phases some run ended in.
				filterOptions: fieldValueOptions(ctx.phaseValues()),
				filterLabel: (id) => discoveryPhases.getName(id),
				getValue: (item) => {
					const phase = ctx.resultsOf(item)?.phase;
					return phase ? discoveryPhases.getName(phase) : null;
				},
				// The server groups on the raw phase id.
				getGroupValue: (item) => ctx.resultsOf(item)?.phase ?? null,
				display: {
					order: 1,
					getItems: (item) => {
						const tag = ctx.outcomeTag(item);
						return tag
							? [{ id: tag.label, label: tag.label, color: tag.color, title: tag.title }]
							: [];
					}
				}
			},
			// Read out of the run's results. Near-unique timestamps and continuous values, so
			// they sort but do not group.
			started_at: {
				type: 'date',
				getValue: (item) => {
					const startedAt = ctx.resultsOf(item)?.started_at;
					return startedAt ? new Date(startedAt) : null;
				},
				display: { order: 3, cell: ctx.startedAtCell() }
			},
			finished_at: {
				type: 'date',
				getValue: (item) => {
					const finishedAt = ctx.resultsOf(item)?.finished_at;
					return finishedAt ? new Date(finishedAt) : null;
				},
				display: { order: 4, cell: ctx.finishedAtCell() }
			},
			duration: {
				type: 'string',
				groupable: false,
				getValue: (item) => {
					const results = ctx.resultsOf(item);
					if (results && results.finished_at && results.started_at) {
						return formatDuration(results.started_at, results.finished_at);
					}
					return common_unknown();
				},
				display: { order: 5 }
			},
			// How much the run had to say, next to how it ended — the two questions asked
			// together. Always amber, never red: this column counts warnings, and a run
			// that failed outright says so in Status. A clean run falls back to its plain
			// 0 rather than wearing a chip that means nothing. A count, so it sorts but does
			// not group.
			warnings: {
				type: 'string',
				groupable: false,
				getValue: (item) => String(ctx.warningsOf(item).length),
				display: {
					order: 2,
					getItems: (item) => {
						const count = ctx.warningsOf(item).length;
						if (count === 0) return undefined;
						return [{ id: 'warnings', label: String(count), color: toColor('amber') }];
					}
				}
			}
		},
		[
			{
				// Light or full, next to Duration: a full scan is why a run took much longer. Read
				// from the run's own scan settings, so it neither sorts nor groups on the server.
				key: 'scan_mode',
				type: 'string',
				getValue: (item) =>
					item.discovery_type.type === 'Unified'
						? isFullScan(item.discovery_type)
							? common_full()
							: common_light()
						: null,
				display: { order: 6 }
			},
			{
				// The server writes runs with no tags, but a tag can be added to one through the API.
				key: 'tags',
				label: common_tags(),
				type: 'array',
				getValue: (item) => tagNames(item.tags, ctx.tags()),
				display: {
					hiddenByDefault: true,
					getItems: (item) => tagItems(item.tags, ctx.tags(), ctx.tagGroups() ?? [])
				}
			}
		],
		// Every run here is historical, so a run-type column would read the same on every row.
		{ omit: ['run_type'] }
	);
}

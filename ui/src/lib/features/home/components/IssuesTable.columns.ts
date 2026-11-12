import type { Snippet } from 'svelte';
import type { components, operations } from '#lib/api/schema';
import { defineFields } from '#lib/shared/components/data/types.ts';
import { siteItems } from '#lib/features/sites/columns.ts';
import type { Site } from '#lib/features/sites/types.ts';
import { issueCategories, setupItems } from '#lib/shared/stores/metadata.ts';
import { common_category, common_issue, common_sites } from '#lib/paraglide/messages.js';
import { issueCategoryOf, type SetupListEntry } from '../setup-list';

type IssueOrderField = components['schemas']['IssueOrderField'];
/** The Issues endpoint's query params, which the filters are sent and linked as. */
type IssueListParam = keyof NonNullable<operations['get_dashboard_issues']['parameters']['query']>;

/** What the Issue columns read from the table, each as a getter so values stay live. */
export interface IssueColumnsContext {
	sites: () => Site[];
	/** The sites the org's Issues are on: the site filter's options. */
	issueSites: () => string[];
	/** The Issue column's content: the row's title, chips and sentence. */
	issueCell: Snippet<[SetupListEntry, string]>;
}

const categoryOf = (entry: SetupListEntry) => issueCategoryOf(entry.item);

/** The Issue types, for the type filter's options: the types whose category is an Issue category. */
function issueTypes(): string[] {
	const categories = new Set(issueCategories.getItems().map((c) => c.id));
	return setupItems
		.getItems()
		.filter((t) => categories.has(t.category ?? ''))
		.map((t) => t.id);
}

/** The Issues table's fields. The server sorts, groups and filters each orderable one. */
export function issueColumns(ctx: IssueColumnsContext) {
	return defineFields<SetupListEntry, IssueOrderField, IssueListParam>(
		{
			type: {
				label: common_issue(),
				type: 'string',
				getValue: (entry) => setupItems.getName(entry.item.type),
				getGroupValue: (entry) => entry.item.type,
				filterable: true,
				serverFiltered: true,
				queryParam: 'types',
				filterOptions: issueTypes(),
				filterLabel: (id) => setupItems.getName(id),
				getFilterValue: (entry) => entry.item.type,
				display: { primary: true, wrap: true, width: 520, cell: ctx.issueCell }
			},
			category: {
				label: common_category(),
				type: 'string',
				getValue: (entry) => issueCategories.getName(categoryOf(entry)),
				getGroupValue: categoryOf,
				filterable: true,
				serverFiltered: true,
				queryParam: 'categories',
				filterOptions: issueCategories.getItems().map((c) => c.id),
				filterLabel: (id) => issueCategories.getName(id),
				getFilterValue: categoryOf,
				display: {
					getItems: (entry) => {
						const id = categoryOf(entry);
						return [
							{
								id,
								label: issueCategories.getName(id),
								icon: issueCategories.getIconComponent(id),
								color: issueCategories.getColorHelper(id).color
							}
						];
					}
				}
			}
		},
		[
			{
				key: 'site_ids',
				label: common_sites(),
				type: 'array',
				getValue: (entry) => siteItems(entry.site_ids, ctx.sites()).map((s) => s.label),
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				filterOptions: ctx.issueSites(),
				filterLabel: (id) => ctx.sites().find((s) => s.id === id)?.name ?? id,
				getFilterValue: (entry) => entry.site_ids,
				display: { getItems: (entry) => siteItems(entry.site_ids, ctx.sites()) }
			}
		]
	);
}

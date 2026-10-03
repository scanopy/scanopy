import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import {
	COVERAGE,
	FEATURES,
	SCHEMAS,
	columnKeys,
	dataControlsTabs,
	definedOrderField,
	orderFieldSchema as tabOrderFieldSchema,
	readTab,
	schemaProperties,
	type TabCoverage
} from './entity-tabs';

/**
 * Every field the API returns for an entity is either a column on that entity's tab or is
 * excluded in `COVERAGE` (`entity-tabs.ts`) with the reason it is not shown.
 *
 * A field the backend adds reaches the frontend through `static/openapi.json` and the generated
 * types, and nothing else asks whether anyone can see it. This test does: the new property has no
 * column and no exclusion, so it fails until someone decides. Most new columns belong in the
 * table with `display: { hiddenByDefault: true }`, so they cost nothing until a user asks.
 */

/** Everything wrong with one tab's coverage, as messages naming the tab, schema and property. */
function coverageProblems(tab: string, coverage: TabCoverage): string[] {
	const source = readTab(tab);
	const problems: string[] = [];

	const declaredOrderField = definedOrderField(source);
	if (declaredOrderField !== coverage.orderField) {
		problems.push(
			`${tab}: the tab calls defineFields with order field "${declaredOrderField ?? 'none'}" ` +
				`but COVERAGE says "${coverage.orderField ?? 'none'}". Set COVERAGE["${tab}"].orderField ` +
				`to match the tab.`
		);
	}
	const orderFieldSchema = tabOrderFieldSchema(coverage);
	if (orderFieldSchema && !SCHEMAS[orderFieldSchema]?.enum) {
		problems.push(`${tab}: "${orderFieldSchema}" is not an enum schema in openapi.json.`);
	}

	const keys = columnKeys(source, orderFieldSchema);
	for (const { schema, aliases = {}, excluded = {} } of coverage.schemas) {
		if (!SCHEMAS[schema]) {
			problems.push(`${tab}: schema "${schema}" does not exist in openapi.json.`);
			continue;
		}
		const where = `${tab} (${schema})`;
		const properties = schemaProperties(schema);

		for (const property of properties) {
			if (!keys.has(property) && !(property in aliases) && !(property in excluded)) {
				problems.push(
					`${where}: "${property}" has no column and no exclusion. Add a column with ` +
						`display: { hiddenByDefault: true } to the tab, or add "${property}" to ` +
						`COVERAGE["${tab}"] ${schema}.excluded with the reason it is not shown.`
				);
			}
		}
		for (const [property, reason] of Object.entries(excluded)) {
			if (!properties.has(property)) {
				problems.push(
					`${where}: excluded "${property}" is not a property of ${schema}. Remove the stale exclusion.`
				);
			}
			if (!reason.trim()) {
				problems.push(
					`${where}: excluded "${property}" has an empty reason. Say why it is not shown.`
				);
			}
			if (keys.has(property) || property in aliases) {
				problems.push(
					`${where}: "${property}" is excluded but is also a column. Remove the exclusion.`
				);
			}
		}
		for (const [property, column] of Object.entries(aliases)) {
			if (!properties.has(property)) {
				problems.push(
					`${where}: alias "${property}" is not a property of ${schema}. Remove the stale alias.`
				);
			}
			if (!keys.has(column)) {
				problems.push(
					`${where}: alias "${property}" points at column "${column}", which the tab does not ` +
						`declare. Point it at the column that renders "${property}", or exclude it instead.`
				);
			}
		}
	}
	return problems;
}

describe('entity column coverage', () => {
	it('has an entry for every tab that renders DataControls', () => {
		const tabs = dataControlsTabs();
		expect(tabs.length).toBeGreaterThan(10);

		const missing = tabs.filter((tab) => !(tab in COVERAGE));
		expect(
			missing,
			`These tabs render <DataControls> with no COVERAGE entry:\n\n  ${missing.join('\n  ')}\n\n` +
				`Add one per tab, naming the schema(s) its rows come from.`
		).toEqual([]);

		const stale = Object.keys(COVERAGE).filter((tab) => !tabs.includes(tab));
		expect(
			stale,
			`COVERAGE entries whose file is missing or no longer renders <DataControls>:\n\n  ` +
				`${stale.join('\n  ')}\n\nUpdate the path or remove the entry.`
		).toEqual([]);
	});

	for (const [tab, coverage] of Object.entries(COVERAGE)) {
		it(`${tab} shows or excludes every schema property`, () => {
			expect(fs.existsSync(path.join(FEATURES, tab)), `${tab} does not exist`).toBe(true);
			const problems = coverageProblems(tab, coverage);
			expect(problems, problems.join('\n')).toEqual([]);
		});
	}
});

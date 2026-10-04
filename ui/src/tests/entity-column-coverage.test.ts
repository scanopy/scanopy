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
	type TabCoverage
} from './entity-tabs';

/**
 * Every field the API returns for an entity is either a column on that entity's tab or is
 * excluded in `COVERAGE` (`entity-tabs.ts`) with the reason it is not shown.
 *
 * `npm run check` enforces the field side: `COVERAGE` is typed against the generated schema, so
 * a field the backend adds fails to compile until someone decides where it shows, and so does a
 * decision for a field that no longer exists or an empty reason. Most new columns belong in the
 * table with `display: { hiddenByDefault: true }`, so they cost nothing until a user asks.
 *
 * This test checks the column side, which types cannot see: each column key a decision names
 * is one the tab declares, so a renamed or removed column can't leave a field decided onto
 * nothing.
 *
 * A reason passes only if the field is a secret, an internal or history id, provably empty on
 * every row the tab lists (name the code path), or a decision recorded with Maya. "The tab
 * doesn't load X", "low value", "another column partly shows it" and "the editor shows it" do
 * not pass: load it, or render it as an entity ref from the id.
 */

/** Everything wrong with one tab's coverage, as messages naming the tab, schema and field. */
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
	for (const { schema, fields } of coverage.schemas) {
		const where = `${tab} (${schema})`;
		for (const [field, decision] of Object.entries(fields)) {
			if (typeof decision !== 'string') {
				if (!decision.excluded.trim()) {
					problems.push(`${where}: "${field}" is excluded with an empty reason. Say why.`);
				}
				if (keys.has(field)) {
					problems.push(
						`${where}: "${field}" is excluded but the tab has a column of that name. Assign ` +
							`it to the column instead.`
					);
				}
			} else if (!keys.has(decision)) {
				problems.push(
					`${where}: "${field}" is shown by column "${decision}", which the tab does not ` +
						`declare. Point it at the column that renders "${field}", or exclude it instead.`
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
		it(`${tab} shows or excludes every schema field`, () => {
			expect(fs.existsSync(path.join(FEATURES, tab)), `${tab} does not exist`).toBe(true);
			const problems = coverageProblems(tab, coverage);
			expect(problems, problems.join('\n')).toEqual([]);
		});
	}
});

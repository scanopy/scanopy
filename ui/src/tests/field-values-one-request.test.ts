import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FEATURES = path.resolve(__dirname, '../lib/features');

/**
 * `useFieldValuesQuery` sends one request per call, and one call counts any number of an entity's
 * fields. A component that calls it twice for the same entity sends two requests where one would
 * do, so each component calls it at most once per field-values path.
 */
function fieldValuesPaths(source: string): string[] {
	return [...source.matchAll(/useFieldValuesQuery\(\s*([^,\s]+)/g)].map(([, argument]) => {
		const literal = argument.match(/^['"](.+)['"]$/);
		if (literal) return literal[1];
		// A path held in a const: read the literal it names.
		const declared = source.match(new RegExp(`const ${argument}\\s*=\\s*['"](.+)['"]`));
		return declared ? declared[1] : argument;
	});
}

describe('field-values requests', () => {
	const components = (fs.readdirSync(FEATURES, { recursive: true }) as string[])
		.filter((file) => file.endsWith('.svelte'))
		.map((file) => ({
			file,
			paths: fieldValuesPaths(fs.readFileSync(path.join(FEATURES, file), 'utf8'))
		}))
		.filter(({ paths }) => paths.length > 0);

	it('finds the components that count field values', () => {
		expect(components.length).toBeGreaterThan(0);
	});

	it('are one per entity in each component', () => {
		const repeated = components.flatMap(({ file, paths }) =>
			[...new Set(paths)]
				.filter((p) => paths.filter((other) => other === p).length > 1)
				.map((p) => `  ${file}  calls useFieldValuesQuery more than once for ${p}`)
		);

		if (repeated.length > 0) {
			expect.fail(
				`${repeated.join('\n')}\n\nName every field in one call: ` +
					`useFieldValuesQuery(path, ['site_id', 'source']).`
			);
		}
	});
});

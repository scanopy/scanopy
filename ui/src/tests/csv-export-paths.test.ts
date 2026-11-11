import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { entityToExportPath } from '#lib/api/entities.ts';

const UI = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const paths = Object.keys(
	JSON.parse(fs.readFileSync(path.join(UI, 'static/openapi.json'), 'utf-8')).paths
);

describe('CSV export paths', () => {
	it('sends every entity with an export to a route the server has', () => {
		for (const [entity, segment] of Object.entries(entityToExportPath)) {
			if (segment === null) continue;
			expect(paths, `${entity} exports to /api/v1/${segment}/export/csv`).toContain(
				`/api/v1/${segment}/export/csv`
			);
		}
	});
});

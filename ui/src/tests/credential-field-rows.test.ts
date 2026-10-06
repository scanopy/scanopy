import { describe, it, expect } from 'vitest';
import { fieldRows } from '$lib/features/credentials/utils/fieldRows';
import credentialTypes from '$lib/data/credential-types.json';

const field = (id: string, half_width = false) => ({ id, half_width });
const ids = (rows: { id: string }[][]) => rows.map((row) => row.map((f) => f.id));

describe('fieldRows', () => {
	it('pairs two adjacent half-width fields and leaves the rest full width', () => {
		const rows = fieldRows([field('port', true), field('wait', true), field('address')]);
		expect(ids(rows)).toEqual([['port', 'wait'], ['address']]);
	});

	it('keeps a half-width field full width when its neighbour is not half width', () => {
		const rows = fieldRows([field('port', true), field('address'), field('wait', true)]);
		expect(ids(rows)).toEqual([['port'], ['address'], ['wait']]);
	});

	it('pairs left to right, so three in a row leave the third alone', () => {
		const rows = fieldRows([field('a', true), field('b', true), field('c', true)]);
		expect(ids(rows)).toEqual([['a', 'b'], ['c']]);
	});
});

describe('credential type field definitions', () => {
	it('flag half-width only on fields that have a half-width neighbour in their group', () => {
		const stranded: string[] = [];
		for (const type of credentialTypes as {
			id: string;
			metadata: { fields: { id: string; group?: string | null; half_width: boolean }[] };
		}[]) {
			const fields = type.metadata.fields;
			fields.forEach((f, i) => {
				if (!f.half_width) return;
				const partner = [fields[i - 1], fields[i + 1]].some(
					(n) => n?.half_width && (n.group ?? null) === (f.group ?? null)
				);
				if (!partner) stranded.push(`${type.id}.${f.id}`);
			});
		}
		expect(stranded).toEqual([]);
	});
});

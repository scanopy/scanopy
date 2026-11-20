import { describe, expect, it, vi } from 'vitest';
import { fetchByIds, IDS_PER_REQUEST } from '#lib/api/query-helpers.ts';

describe('fetchByIds', () => {
	it('splits a long id list into requests of at most IDS_PER_REQUEST and keeps every row', async () => {
		const ids = Array.from({ length: IDS_PER_REQUEST * 2 + 1 }, (_, i) => `id-${i}`);
		const fetchBatch = vi.fn(async (batch: string[]) => batch.map((id) => ({ id })));

		const rows = await fetchByIds(ids, fetchBatch);

		expect(fetchBatch).toHaveBeenCalledTimes(3);
		for (const [batch] of fetchBatch.mock.calls) {
			expect(batch.length).toBeLessThanOrEqual(IDS_PER_REQUEST);
		}
		expect(rows.map((r) => r.id)).toEqual(ids);
	});

	it('sends no request for no ids', async () => {
		const fetchBatch = vi.fn(async () => []);
		expect(await fetchByIds([], fetchBatch)).toEqual([]);
		expect(fetchBatch).not.toHaveBeenCalled();
	});
});

import type { components } from '$lib/api/schema';
import elementSortsJson from '$lib/data/element-sorts.json';
import type { TopologyOptions } from './types/base';

export type ElementSort = components['schemas']['ElementSort'];

type ElementSortMetadata = (typeof elementSortsJson)[number];

/** The sorts a view offers, in the order the backend declares them. */
export function elementSortsFor(view: string): ElementSortMetadata[] {
	return elementSortsJson.filter((s) => s.metadata.views.includes(view));
}

/**
 * The sort a view is built with: the stored choice if the view offers it, otherwise `Automatic`.
 * Mirrors `GroupingConfig::from_request_options`, which drops a sort the view cannot use.
 */
export function effectiveElementSort(
	request: TopologyOptions['request'] | undefined,
	view: string
): ElementSort {
	const stored = request?.element_sort?.[view];
	if (stored && elementSortsFor(view).some((s) => s.id === stored)) return stored;
	return 'Automatic';
}

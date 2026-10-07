/**
 * The colours a view paints on its element cards: a title, a port's status dot, a zoomed-out fill.
 *
 * A colour on the canvas is shown only when the view gives the user a way to read it. Each view
 * declares its colours as element marks (`element_marks` in the views fixture), and every mark
 * names a filter value the view offers in its filter panel. The colour is that value's chip
 * colour, and a card carries it when the value's extractor, the same one the chip's hover ring
 * uses, returns that value for the card. Nothing here picks a colour of its own, so a card colour
 * without a chip to explain it has nowhere to come from. L3's amber host titles went this way:
 * L3 offers no Host filter, so it paints no Host title.
 *
 * # Why the zoomed-out fill is state and not identity
 *
 * Below the detail threshold a card draws as its box, and a white box is noise. The tempting
 * sources are identity palettes, and they all fail the same way: entity type is constant within a
 * view, and a service definition's colour per instance is a rainbow, not information. So the fill
 * is a small ordered vocabulary (down, stale, up) or the neutral token.
 */

import type { components } from '$lib/api/schema';
import type { Site } from '$lib/features/sites/types';
import { createColorHelper, type Color } from '$lib/shared/utils/styling';
import { FILTER_VALUE_EXTRACTORS } from './interactions';
import { cardEntityForFilter, type ElementRenderContext } from './resolvers';
import type { RenderableTopology } from './types/base';
import { viewElementConfig, type ViewElementConfig } from './view-filters';

type ElementMark = components['schemas']['ElementMark'];
export type MarkChannel = components['schemas']['MarkChannel'];

/** The colour each channel paints on one card; an absent channel renders neutral. */
export type ElementMarks = Partial<Record<MarkChannel, Color>>;

/** A reduced card with no mark: neutral texture, tuned per theme in `app.css`. */
export const NEUTRAL_FILL = 'var(--color-topology-lod-unknown)';
/** A status dot with no mark. */
export const NEUTRAL_DOT = 'rgb(156, 163, 175)';

/** The chip colour of the filter value `mark` names, or undefined if the view does not offer it. */
export function markColor(config: ViewElementConfig, mark: ElementMark): Color | undefined {
	return config.metadata_filters
		?.find((f) => f.filter_type === mark.filter_type && f.entities.includes(mark.entity))
		?.values.find((v) => v.id === mark.value)?.color;
}

/**
 * The colours `view` paints on the card `resolved` describes. Per channel the first matching mark
 * wins, which is how L2 puts a down port above a stale one.
 */
export function elementMarks(
	view: string,
	resolved: ElementRenderContext,
	sites: Site[],
	topology: RenderableTopology
): ElementMarks {
	const config = viewElementConfig(view);
	const marks: ElementMarks = {};
	if (!config) return marks;
	for (const mark of config.element_marks ?? []) {
		if (marks[mark.channel]) continue;
		const entity = cardEntityForFilter(resolved, mark.entity);
		const extract = FILTER_VALUE_EXTRACTORS[mark.entity]?.[mark.filter_type];
		if (!entity || !extract) continue;
		const site = sites.find((n) => n.id === (entity as { site_id?: string }).site_id);
		if (extract(entity, { site, topology }) !== mark.value) continue;
		const color = markColor(config, mark);
		if (color) marks[mark.channel] = color;
	}
	return marks;
}

/** CSS colour for a reduced card's fill. */
export function stateFill(marks: ElementMarks): string {
	return marks.StateFill ? createColorHelper(marks.StateFill).rgb : NEUTRAL_FILL;
}

/** CSS colour for a port card's status dot. */
export function statusDot(marks: ElementMarks): string {
	return marks.StatusDot ? createColorHelper(marks.StatusDot).rgb : NEUTRAL_DOT;
}

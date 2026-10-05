import type { TagProps } from '../../data/types';
import type { DisplayTagContext, EntityDisplayComponent } from './types';

/**
 * The tags a display shows in this context: its `getTags`, less the roles the context hides
 * (`hideTags`, plus the display's `compactHides` when `compact`). Tags without a role always show.
 */
export function displayTags<T, C>(
	displayComponent: EntityDisplayComponent<T, C>,
	item: T,
	context: C
): TagProps[] {
	const tags = displayComponent.getTags?.(item, context) ?? [];
	const tagContext = (context && typeof context === 'object' ? context : {}) as DisplayTagContext;
	const hidden = new Set<string>([
		...(tagContext.hideTags ?? []),
		...(tagContext.compact ? (displayComponent.compactHides ?? []) : [])
	]);
	if (hidden.size === 0) return tags;
	return tags.filter((tag) => !tag.role || !hidden.has(tag.role));
}

export interface TagFitSpacing {
	/** Between the label and the tag group. */
	gap: number;
	/** Between two tags, and before the "+N tags" chip. */
	tagGap: number;
	/** Width of the hidden-tags (i) icon. */
	moreWidth: number;
}

/** Width of the (i) icon `HiddenTagsChip` draws for hidden tags: a 14px icon plus its padding. */
export const HIDDEN_TAGS_CHIP_WIDTH = 18;

/**
 * How many tags fit beside a label at its full width. The label is the row's key value, so tags
 * take only the space it leaves, and the count can be 0. When some tags don't fit, room is kept
 * for the (i) icon that stands in for them.
 */
export function fitTags(
	containerWidth: number,
	labelWidth: number,
	tagWidths: readonly number[],
	spacing: TagFitSpacing
): number {
	const available = containerWidth - labelWidth - spacing.gap;
	let count = 0;
	let used = 0;
	for (let i = 0; i < tagWidths.length; i++) {
		const next = used + (count > 0 ? spacing.tagGap : 0) + tagWidths[i];
		const moreRemain = i < tagWidths.length - 1;
		const needed = next + (moreRemain ? spacing.tagGap + spacing.moreWidth : 0);
		if (needed > available) break;
		count++;
		used = next;
	}
	return count;
}

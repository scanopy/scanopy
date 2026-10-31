import type { EntityRef } from '$lib/shared/components/data/types';
import type { Color } from '$lib/shared/utils/styling';
import type { IconComponent } from '$lib/shared/utils/types';
import {
	hideTooltipPopover,
	showTooltipPopover,
	tooltipChipsAreStatic
} from './tooltip-popover.svelte';

/**
 * Svelte action that shows a tooltip on hover.
 * Shows `content` when given, otherwise the plain text of the element's data-tooltip attribute.
 * Rendered in a `Popover`, so it places, sizes and looks the same as every other popover.
 *
 * The app has one tooltip. The action only registers its element; one pair of pointer listeners on
 * the document finds the nearest registered element under the pointer and moves the tooltip there,
 * so a page of a few thousand topology cards carries no listener per card. Nested registered
 * elements show the innermost one.
 *
 * `anchorClosest` places the tooltip against the nearest ancestor matching that selector instead
 * of the node itself, so a tooltip on part of a card clears the whole card.
 */

/** A value drawn as the app draws it elsewhere: a `Tag`, or an `EntityTag` for a named entity. */
export type TooltipChip =
	| { type: 'Tag'; label: string; color: Color; icon?: IconComponent | null }
	| { type: 'Entity'; label: string; color: Color; ref: EntityRef };

/** One labelled fact: a muted label and an emphasized value, drawn as `chips` when it has them. */
export interface TooltipRow {
	label: string;
	/** The plain text of the value. */
	value: string;
	chips?: TooltipChip[];
}

/** A list entry with a muted detail set apart on its right: a service and its ports. */
export interface TooltipItem {
	text: string;
	detail?: string;
	/** Draws the entry in place of `text`, which stays its plain form. */
	chip?: TooltipChip;
}

/** A block of a structured tooltip, set off from the one above it by a rule. */
export interface TooltipSection {
	heading?: string;
	/** A muted sentence or two: a description, or why something is the way it is. */
	text?: string;
	rows?: TooltipRow[];
	items?: (string | TooltipItem)[];
}

/** A tooltip with structure: an optional title, then its sections in order. */
export interface TooltipContent {
	title?: string;
	sections: TooltipSection[];
}

/** A tooltip's text as one name: plain text as it is, structured content by its title. */
export function tooltipLabel(content: string | TooltipContent | null | undefined): string {
	return typeof content === 'string' ? content : (content?.title ?? '');
}

export interface TooltipOptions {
	/** The content, or a function that builds it when the tooltip opens. */
	content?: TooltipContent | null | (() => TooltipContent | null);
	anchorClosest?: string;
}

/** The part of `Document` the renderer uses, so it renders the same into any document. */
type TooltipDocument = Pick<Document, 'createElement'>;

function el(
	doc: TooltipDocument,
	tag: string,
	style: Partial<CSSStyleDeclaration>,
	text?: string
): HTMLElement {
	const e = doc.createElement(tag);
	Object.assign(e.style, style);
	if (text !== undefined) e.textContent = text;
	return e;
}

/** Mounts a chip's component into the empty element the renderer made for it. */
export type MountTooltipChip = (target: HTMLElement, chip: TooltipChip) => void;

/** A chip's place in the tooltip: `mountChip` fills it, so no chip text passes through here. */
function chipSlot(doc: TooltipDocument, chip: TooltipChip, mountChip: MountTooltipChip) {
	const slot = el(doc, 'span', { display: 'inline-flex', maxWidth: '100%', minWidth: '0' });
	mountChip(slot, chip);
	return slot;
}

/** Whether the tooltip holds a chip that navigates, so the pointer has to be able to reach it. */
export function hasEntityChips(content: string | TooltipContent): boolean {
	if (typeof content === 'string') return false;
	const isEntity = (chip?: TooltipChip) => chip?.type === 'Entity';
	return content.sections.some(
		(section) =>
			section.rows?.some((row) => row.chips?.some(isEntity)) ||
			section.items?.some((item) => typeof item !== 'string' && isEntity(item.chip))
	);
}

/**
 * The tooltip's body. Every string, which may come from a scanned device or a user (a hostname, a
 * service or tag name), is set through `textContent`, so markup in it renders as text. Chips are
 * `Tag` and `EntityTag` components, which render their labels as text too.
 */
export function renderTooltip(
	doc: TooltipDocument,
	content: string | TooltipContent,
	mountChip: MountTooltipChip
): HTMLElement {
	if (typeof content === 'string') {
		// Keeps the line breaks of multi-line text.
		return el(doc, 'div', { whiteSpace: 'pre-line' }, content);
	}

	const root = el(doc, 'div', { display: 'flex', flexDirection: 'column', gap: '6px' });
	if (content.title) {
		root.appendChild(
			el(
				doc,
				'div',
				{ color: 'var(--color-text-primary)', fontWeight: '600', fontSize: '13px' },
				content.title
			)
		);
	}
	content.sections.forEach((section, i) => {
		const block = el(doc, 'div', { display: 'flex', flexDirection: 'column', gap: '4px' });
		if (i > 0 || content.title) {
			Object.assign(block.style, {
				borderTop: '1px solid var(--color-border)',
				paddingTop: '6px'
			});
		}
		if (section.heading) {
			block.appendChild(
				el(
					doc,
					'div',
					{
						color: 'var(--color-text-tertiary)',
						fontSize: '10px',
						fontWeight: '600',
						letterSpacing: '0.05em',
						textTransform: 'uppercase'
					},
					section.heading
				)
			);
		}
		if (section.text) {
			block.appendChild(el(doc, 'div', { color: 'var(--color-text-secondary)' }, section.text));
		}
		if (section.rows?.length) {
			const grid = el(doc, 'div', {
				display: 'grid',
				gridTemplateColumns: 'auto 1fr',
				columnGap: '12px',
				rowGap: '2px'
			});
			for (const row of section.rows) {
				grid.appendChild(el(doc, 'div', { color: 'var(--color-text-tertiary)' }, row.label));
				const value = el(doc, 'div', {
					color: 'var(--color-text-primary)',
					fontWeight: '500',
					display: 'flex',
					flexWrap: 'wrap',
					alignItems: 'center',
					gap: '4px',
					minWidth: '0'
				});
				if (row.chips?.length) {
					for (const chip of row.chips) value.appendChild(chipSlot(doc, chip, mountChip));
				} else {
					value.appendChild(el(doc, 'span', { overflowWrap: 'anywhere' }, row.value));
				}
				grid.appendChild(value);
			}
			block.appendChild(grid);
		}
		if (section.items?.length) {
			const list = el(doc, 'ul', {
				margin: '0',
				paddingLeft: '14px',
				listStyleType: 'disc',
				color: 'var(--color-text-primary)'
			});
			for (const item of section.items) {
				const chip = typeof item === 'string' ? undefined : item.chip;
				if (typeof item === 'string' || !item.detail) {
					const li = el(
						doc,
						'li',
						{},
						chip ? undefined : typeof item === 'string' ? item : item.text
					);
					if (chip) li.appendChild(chipSlot(doc, chip, mountChip));
					list.appendChild(li);
					continue;
				}
				// The detail sits right-aligned and muted, so it never reads as part of the name. The name
				// keeps its width and breaks only between words (its min-width stays at its longest word);
				// a long detail such as a port list takes the rest of the row and wraps onto more lines.
				const li = el(doc, 'li', {});
				const line = el(doc, 'div', {
					display: 'flex',
					justifyContent: 'space-between',
					alignItems: 'baseline',
					gap: '12px'
				});
				const name = el(
					doc,
					'span',
					{ flex: '0 1 auto', maxWidth: '60%' },
					chip ? undefined : item.text
				);
				if (chip) name.appendChild(chipSlot(doc, chip, mountChip));
				line.appendChild(name);
				line.appendChild(
					el(
						doc,
						'span',
						{
							color: 'var(--color-text-tertiary)',
							fontSize: '11px',
							fontVariantNumeric: 'tabular-nums',
							textAlign: 'right',
							flex: '1 1 0',
							minWidth: '0',
							overflowWrap: 'anywhere'
						},
						item.detail
					)
				);
				li.appendChild(line);
				list.appendChild(li);
			}
			block.appendChild(list);
		}
		root.appendChild(block);
	});
	return root;
}

/** The part of an element the hover tracking reads, so it runs the same against any tree. */
export interface TooltipTarget {
	closest(selector: string): TooltipTarget | null;
}

/** Marks a registered element, so the nearest one under the pointer is a `closest` away. */
const SOURCE_ATTRIBUTE = 'data-tooltip-source';

/** Marks the tooltip's own surface, which holds chips of its own. */
export const SURFACE_ATTRIBUTE = 'data-tooltip-surface';

/** How long a tooltip with chips to click stays after the pointer leaves, to cross the gap to it. */
const REACH_GRACE_MS = 150;

/**
 * Which registered element the tooltip is on, as the pointer moves. `pointerover` and `pointerout`
 * both name where the pointer went; the tooltip follows to the nearest registered element there,
 * or closes when there is none. Moving between two registered elements switches straight from one
 * to the other, so the one tooltip moves rather than closing and reopening.
 *
 * `reachable` says the shown tooltip holds something to click. The pointer can then cross to the
 * tooltip and stay on it, and chips inside it never become sources, so a nested chip opens no
 * tooltip of its own.
 */
export class TooltipTracker<T extends TooltipTarget> {
	current: T | null = null;
	private closing: ReturnType<typeof setTimeout> | undefined;

	constructor(
		private readonly onShow: (source: T) => void,
		private readonly onHide: () => void,
		private readonly reachable: () => boolean = () => false
	) {}

	/** The pointer is now over `target` (null: it left the document). */
	moveTo(target: T | null) {
		const onSurface = target?.closest(`[${SURFACE_ATTRIBUTE}]`) ?? null;
		if (onSurface && this.current && this.reachable()) {
			clearTimeout(this.closing);
			return;
		}
		const source = (target?.closest(`[${SOURCE_ATTRIBUTE}]`) as T | null) ?? null;
		clearTimeout(this.closing);
		if (source === this.current) return;
		if (!source && this.reachable()) {
			this.closing = setTimeout(() => this.dismiss(), REACH_GRACE_MS);
			return;
		}
		this.current = source;
		if (source) this.onShow(source);
		else this.onHide();
	}

	/** Closes the tooltip now, as a click on a chip inside it does. */
	dismiss() {
		clearTimeout(this.closing);
		this.current = null;
		this.onHide();
	}

	/**
	 * The pointer left for `related` (null: it left the document). A touch has no hover: its
	 * pointer leaves the moment the tap ends, so a tapped tooltip stays until the next tap lands
	 * elsewhere (a `moveTo` there, or the popover's outside click).
	 */
	leave(related: T | null, pointerType: string) {
		if (pointerType === 'touch') return;
		this.moveTo(related);
	}

	/** A registered element is going away or changed: close or redraw if it is the one shown. */
	refresh(source: T, removed: boolean) {
		if (source !== this.current) return;
		if (removed) {
			this.dismiss();
		} else {
			this.onShow(source);
		}
	}
}

const sources = new WeakMap<HTMLElement, TooltipOptions>();

function contentOf(node: HTMLElement): string | TooltipContent | null {
	const options = sources.get(node);
	const given = typeof options?.content === 'function' ? options.content() : options?.content;
	return given ?? node.getAttribute('data-tooltip');
}

/** Whether the tooltip now shown holds a chip to click. */
let reachable = false;

const tracker = new TooltipTracker<HTMLElement>(
	(node) => {
		const content = contentOf(node);
		reachable = !!content && hasEntityChips(content) && !tooltipChipsAreStatic();
		if (!content) {
			hideTooltipPopover();
			return;
		}
		const anchorClosest = sources.get(node)?.anchorClosest;
		const anchor = (anchorClosest && node.closest<HTMLElement>(anchorClosest)) || node;
		showTooltipPopover(anchor, content, (c, mountChip) => renderTooltip(document, c, mountChip));
	},
	() => {
		reachable = false;
		hideTooltipPopover();
	},
	() => reachable
);

/** Closes the tooltip, for a click that navigates away from what it is on. */
export function dismissTooltip() {
	tracker.dismiss();
}

let listening = false;

function listen() {
	if (listening) return;
	listening = true;
	document.addEventListener('pointerover', (e) => tracker.moveTo(e.target as HTMLElement));
	document.addEventListener('pointerout', (e) => {
		// Leaving the window has no related target: the tooltip closes.
		tracker.leave((e.relatedTarget as HTMLElement | null) ?? null, e.pointerType);
	});
}

export function tooltip(node: HTMLElement, options: TooltipOptions = {}) {
	sources.set(node, options);
	node.setAttribute(SOURCE_ATTRIBUTE, '');
	listen();

	return {
		update(next: TooltipOptions = {}) {
			sources.set(node, next);
			// A tooltip open while its content changes redraws with the new content.
			tracker.refresh(node, false);
		},
		destroy() {
			sources.delete(node);
			node.removeAttribute(SOURCE_ATTRIBUTE);
			tracker.refresh(node, true);
		}
	};
}

import { createRawSnippet, mount, unmount, untrack } from 'svelte';
import Popover from '$lib/shared/components/data/Popover.svelte';
import TooltipChipView from '$lib/shared/components/data/TooltipChip.svelte';
import type { MountTooltipChip, TooltipContent } from './tooltip';

/**
 * The app's one tooltip: a single `Popover`, mounted at the first hover and kept. Showing it on
 * another element moves it there and swaps its content; it is never mounted twice. It is read-only:
 * its chips stay static, and the pointer passes through it to whatever is underneath.
 */

interface Shown {
	anchor: HTMLElement;
	content: string | TooltipContent;
}

const state = $state<{ shown: Shown | null }>({ shown: null });
let mounted = false;

export function showTooltipPopover(
	anchor: HTMLElement,
	content: string | TooltipContent,
	render: (content: string | TooltipContent, mountChip: MountTooltipChip) => HTMLElement
) {
	state.shown = { anchor, content };
	if (mounted) return;
	mounted = true;

	const body = createRawSnippet(() => ({
		render: () => '<div class="text-secondary wrap-break-word text-xs leading-tight"></div>',
		setup(el: Element) {
			// Redrawn in place whenever the tooltip moves to another element.
			$effect(() => {
				const shown = state.shown;
				if (!shown) return;
				const chips: ReturnType<typeof mount>[] = [];
				const context = new Map([['staticTags', true]]);
				untrack(() => {
					el.replaceChildren(
						render(shown.content, (target, chip) => {
							chips.push(mount(TooltipChipView, { target, props: { chip }, context }));
						})
					);
				});
				return () => {
					for (const chip of chips) unmount(chip);
				};
			});
		}
	}));

	const host = document.createElement('div');
	document.body.appendChild(host);
	mount(Popover, {
		target: host,
		props: {
			get triggerElement() {
				return state.shown?.anchor ?? null;
			},
			get isOpen() {
				return state.shown !== null;
			},
			get maxWidth() {
				return typeof state.shown?.content === 'string' ? '250px' : '300px';
			},
			role: 'tooltip',
			align: 'start',
			minWidth: '0',
			passthrough: true,
			onClose: hideTooltipPopover,
			children: body
		}
	});
}

export function hideTooltipPopover() {
	state.shown = null;
}

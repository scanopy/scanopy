<script lang="ts">
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { fitToViewport } from '$lib/shared/utils/popoverPosition';

	let {
		triggerElement = null,
		isOpen = false,
		onClose,
		onMouseEnter,
		onMouseLeave,
		role = 'tooltip',
		ariaLabel = undefined,
		id = undefined,
		align = 'center',
		prefer = 'below',
		minWidth = '200px',
		maxWidth = '350px',
		matchTriggerWidth = false,
		passthrough = false,
		children
	}: {
		triggerElement?: HTMLElement | null;
		isOpen?: boolean;
		onClose: () => void;
		onMouseEnter?: () => void;
		onMouseLeave?: () => void;
		/**
		 * `dialog` for a popover holding controls: it takes focus when it opens,
		 * hands it back to the trigger on Escape, and closes when focus leaves it.
		 * The popover is portalled to the end of the body, so without this a
		 * keyboard user could open it and never reach what is inside.
		 *
		 * `presentation` for a list whose trigger keeps focus, such as a select's
		 * search input or a combobox: the list inside carries the semantics.
		 */
		role?: 'tooltip' | 'dialog' | 'presentation';
		ariaLabel?: string;
		/** For a trigger's `aria-controls`. */
		id?: string;
		/** `start` lines the popover up with the trigger's left edge; `center` centres it. */
		align?: 'start' | 'center';
		/** The side it opens on when the content fits there or that side has the more room. */
		prefer?: 'below' | 'above';
		minWidth?: string;
		/** CSS max-width. A popover shown only while hovered can take the viewport width, so its
		 *  content needs no truncation. */
		maxWidth?: string;
		/** Exactly as wide as the trigger, as a select's options are; overrides the min and max. */
		matchTriggerWidth?: boolean;
		/** Read-only content: the pointer passes through to whatever is underneath. */
		passthrough?: boolean;
		children: Snippet;
	} = $props();

	const FOCUSABLE = 'button:not([disabled]), input:not([disabled]), select, [tabindex]';

	let portalContainer: HTMLDivElement | null = $state(null);
	let popoverEl: HTMLDivElement | undefined = $state();
	let contentEl: HTMLDivElement | undefined = $state();

	onMount(() => {
		portalContainer = document.createElement('div');
		portalContainer.style.position = 'absolute';
		portalContainer.style.top = '0';
		portalContainer.style.left = '0';
		portalContainer.style.width = '0';
		portalContainer.style.height = '0';
		// A tooltip is mounted once and kept, so a dropdown opened later sits after it in the body;
		// the higher layer keeps a tooltip on a dropdown row above the dropdown.
		portalContainer.style.zIndex = role === 'tooltip' ? '10000' : '9999';
		portalContainer.dataset.popoverPortal = '';
		document.body.appendChild(portalContainer);

		return () => {
			portalContainer?.remove();
		};
	});

	function portal(node: HTMLElement) {
		if (portalContainer) {
			portalContainer.appendChild(node);
		}
		return {
			destroy() {
				// Cleaned up when portalContainer is removed
			}
		};
	}

	/**
	 * Below the trigger, or above it when the content doesn't fit below and there is more room
	 * above; as tall as the content, up to the viewport margin on that side. Set on the element
	 * directly: measuring the uncapped height clears the cap for one synchronous read. The
	 * template's `style:` directives set single properties, so they leave these alone.
	 */
	function updatePosition() {
		if (!triggerElement || !popoverEl) return;
		const rect = triggerElement.getBoundingClientRect();
		const style = popoverEl.style;
		if (matchTriggerWidth) style.minWidth = style.maxWidth = style.width = `${rect.width}px`;
		style.maxHeight = 'none';
		const fit = fitToViewport({
			trigger: rect,
			contentHeight: popoverEl.offsetHeight,
			width: popoverEl.offsetWidth,
			viewport: { width: window.innerWidth, height: window.innerHeight },
			align,
			prefer
		});
		style.top = fit.top === null ? '' : `${fit.top}px`;
		style.bottom = fit.bottom === null ? '' : `${fit.bottom}px`;
		style.left = `${fit.left}px`;
		style.maxHeight = `${fit.maxHeight}px`;
	}

	$effect(() => {
		if (isOpen && popoverEl && role === 'dialog') {
			popoverEl.querySelector<HTMLElement>(FOCUSABLE)?.focus();
		}
	});

	// Follow the trigger as the page scrolls or resizes, and refit as the content or trigger changes.
	$effect(() => {
		if (!isOpen || !triggerElement || !popoverEl || !contentEl) return;
		updatePosition();

		function handleScroll(e: Event) {
			// A scrollable list inside the popover scrolls too; that moves nothing.
			if (popoverEl?.contains(e.target as Node)) return;
			updatePosition();
		}

		// Refit on the next frame, not inside the delivery: the fit sets the popover's width and
		// height, which resizes the observed content and would raise the ResizeObserver loop error.
		let frame = 0;
		const resizeObserver = new ResizeObserver(() => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(updatePosition);
		});
		resizeObserver.observe(contentEl);
		resizeObserver.observe(triggerElement);
		window.addEventListener('scroll', handleScroll, true);
		window.addEventListener('resize', updatePosition);

		return () => {
			resizeObserver.disconnect();
			cancelAnimationFrame(frame);
			window.removeEventListener('scroll', handleScroll, true);
			window.removeEventListener('resize', updatePosition);
		};
	});

	/**
	 * Whether `node` sits in a popover opened from inside this one, such as a tag picker's
	 * suggestions inside a popover holding the picker. Portalled, it is outside this popover's
	 * element, but a click or focus there must not close this one: it would unmount the picker
	 * before the click lands. A nested popover mounts after its parent, so its portal follows.
	 */
	function inNestedPopover(node: Node | null): boolean {
		const host = node instanceof Element ? node.closest('[data-popover-portal]') : null;
		return (
			!!host &&
			!!portalContainer &&
			host !== portalContainer &&
			!!(portalContainer.compareDocumentPosition(host) & Node.DOCUMENT_POSITION_FOLLOWING)
		);
	}

	function handleFocusOut(e: FocusEvent) {
		if (role !== 'dialog') return;
		// No related target is a click on non-focusable content, such as a
		// checkbox's label text, or a focused row moved in the DOM; a click
		// outside is handled on mousedown.
		const next = e.relatedTarget as Node | null;
		if (
			!next ||
			popoverEl?.contains(next) ||
			triggerElement?.contains(next) ||
			inNestedPopover(next)
		)
			return;
		onClose();
	}

	// Close on click outside or Escape
	$effect(() => {
		if (!isOpen) return;

		function handleClickOutside(e: MouseEvent) {
			if (
				popoverEl &&
				!popoverEl.contains(e.target as Node) &&
				triggerElement &&
				!triggerElement.contains(e.target as Node) &&
				!inNestedPopover(e.target as Node)
			) {
				onClose();
			}
		}

		function handleKeydown(e: KeyboardEvent) {
			if (e.key !== 'Escape') return;
			onClose();
			if (role === 'dialog') {
				// Escape closes the innermost layer only. A modal listens on `window`, which this
				// `document` listener runs before, so without this one press closed both.
				e.stopPropagation();
				triggerElement?.focus();
			}
		}

		document.addEventListener('mousedown', handleClickOutside);
		document.addEventListener('keydown', handleKeydown);

		return () => {
			document.removeEventListener('mousedown', handleClickOutside);
			document.removeEventListener('keydown', handleKeydown);
		};
	});
</script>

{#if isOpen && portalContainer}
	<!-- The surface clips to its radius; the inner box scrolls, so the scrollbar meets the border.
	     A child that should scroll on its own, below controls that stay put, takes `min-h-0` and
	     `overscroll-contain`, or a scroll that reaches its end carries on into this box. -->
	<div
		use:portal
		bind:this={popoverEl}
		{id}
		class="fixed z-9999 flex flex-col overflow-hidden rounded-lg border shadow-[0_4px_24px_rgba(0,0,0,0.15)] dark:shadow-[0_4px_24px_rgba(0,0,0,0.5)]"
		style:border-color="var(--color-border)"
		style:background="var(--color-bg-elevated)"
		style:min-width={minWidth}
		style:max-width={maxWidth}
		style:pointer-events={passthrough ? 'none' : undefined}
		{role}
		aria-label={ariaLabel}
		onfocusout={handleFocusOut}
		onmouseenter={onMouseEnter}
		onmouseleave={onMouseLeave}
	>
		<div bind:this={contentEl} class="flex min-h-0 flex-col overflow-y-auto overscroll-contain p-2">
			{@render children()}
		</div>
	</div>
{/if}

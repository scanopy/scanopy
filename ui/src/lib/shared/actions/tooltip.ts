/**
 * Svelte action that shows a fixed-position tooltip on hover.
 * Reads tooltip text from the element's data-tooltip attribute.
 * Uses position:fixed and appends to document.body to escape overflow containers.
 */
export function tooltip(node: HTMLElement) {
	let tip: HTMLDivElement | null = null;

	function show() {
		const text = node.getAttribute('data-tooltip');
		if (!text) return;

		tip = document.createElement('div');
		tip.textContent = text;
		Object.assign(tip.style, {
			position: 'fixed',
			zIndex: '9999',
			maxWidth: '250px',
			padding: '6px 10px',
			borderRadius: '6px',
			fontSize: '12px',
			lineHeight: '1.25',
			color: 'rgb(229 231 235)',
			background: 'rgb(17 24 39)',
			border: '1px solid rgb(55 65 81)',
			boxShadow: '0 4px 6px -1px rgb(0 0 0 / 0.3)',
			pointerEvents: 'none',
			// Keeps the line breaks of multi-line text (an OS tag's details, one per line).
			whiteSpace: 'pre-line',
			wordWrap: 'break-word',
			transform: 'translateX(-50%) translateY(-100%)'
		});
		document.body.appendChild(tip);

		const rect = node.getBoundingClientRect();
		const tipHeight = tip.offsetHeight;

		// Auto-detect: if tooltip would overflow above viewport, position below instead
		if (rect.top - tipHeight - 6 < 0) {
			tip.style.transform = 'translateX(-50%)';
			tip.style.top = `${rect.bottom + 6}px`;
		} else {
			tip.style.top = `${rect.top - 6}px`;
		}

		// The tooltip is centred on the node via translateX(-50%). Clamp that centre
		// so a wide tooltip on an edge (first/last) element stays fully in the
		// viewport instead of being clipped off the left/right edge.
		const margin = 6;
		const half = tip.offsetWidth / 2;
		const centre = rect.left + rect.width / 2;
		const clamped = Math.max(margin + half, Math.min(centre, window.innerWidth - margin - half));
		tip.style.left = `${clamped}px`;
	}

	function hide() {
		tip?.remove();
		tip = null;
	}

	node.addEventListener('mouseenter', show);
	node.addEventListener('mouseleave', hide);

	return {
		destroy() {
			node.removeEventListener('mouseenter', show);
			node.removeEventListener('mouseleave', hide);
			tip?.remove();
		}
	};
}

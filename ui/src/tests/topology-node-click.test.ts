/**
 * A click on a node at simplified zoom. A container drawn as a box has no title chip, and the
 * click handler only selected a container through its title, so a boxed container could not be
 * selected at all: every click on one cleared the selection.
 */
import { describe, it, expect } from 'vitest';
import { resolveNodeClick } from '#lib/features/topology/selection.ts';
import type { NodeDetail } from '#lib/features/topology/pipeline/render-mode.ts';

/** Containers by id: what each draws and its parent. */
function graph(containers: Record<string, { tier: NodeDetail; parent?: string }>) {
	return {
		tierOf: (id: string) => containers[id]?.tier,
		parentOf: (id: string) => containers[id]?.parent
	};
}

const click = (
	nodeId: string,
	g: ReturnType<typeof graph>,
	opts: { isContainer?: boolean; viewportMoved?: boolean; onTitle?: boolean } = {}
) =>
	resolveNodeClick({
		nodeId,
		isContainer: opts.isContainer ?? true,
		viewportMoved: opts.viewportMoved ?? false,
		onTitle: opts.onTitle ?? false,
		...g
	});

describe('resolveNodeClick', () => {
	const simplified = graph({
		site: { tier: 'boxed' },
		host: { tier: 'boxed', parent: 'site' },
		ports: { tier: 'hidden', parent: 'host' },
		portGroup: { tier: 'hidden', parent: 'ports' }
	});

	it('selects a container drawn as a box when its body is clicked', () => {
		expect(click('host', simplified)).toEqual({ type: 'select', id: 'host' });
	});

	it('does nothing when the click ends a drag that panned the view', () => {
		expect(click('host', simplified, { viewportMoved: true })).toEqual({ type: 'ignore' });
	});

	it('selects the card, not its container, when a boxed card is clicked', () => {
		expect(click('card', simplified, { isContainer: false })).toEqual({
			type: 'select',
			id: 'card'
		});
	});

	it('sends a hidden subcontainer’s click to the nearest container that is drawn', () => {
		expect(click('portGroup', simplified)).toEqual({ type: 'select', id: 'host' });
	});

	it('treats a body click at full detail as a canvas click', () => {
		const full = graph({ site: { tier: 'full' } });
		expect(click('site', full)).toEqual({ type: 'pane' });
		expect(click('site', full, { onTitle: true })).toEqual({ type: 'select', id: 'site' });
	});

	it('follows the drawn ancestor’s rule when that ancestor is at full detail', () => {
		// A selected container keeps its header at any zoom while its children stay reduced.
		const kept = graph({
			host: { tier: 'full' },
			ports: { tier: 'hidden', parent: 'host' }
		});
		expect(click('ports', kept)).toEqual({ type: 'pane' });
	});

	it('treats a container it cannot read a tier for as a canvas click', () => {
		expect(click('unmounted', graph({}))).toEqual({ type: 'pane' });
	});
});

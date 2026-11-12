import { describe, it, expect } from 'vitest';
import { TooltipTracker, type TooltipTarget } from '#lib/shared/actions/tooltip.ts';

/** An element in a tree, as far as the tracker reads one: whether it or an ancestor is registered. */
class FakeTarget implements TooltipTarget {
	constructor(
		readonly name: string,
		readonly parent: FakeTarget | null = null,
		readonly registered = false
	) {}
	closest(): FakeTarget | null {
		return this.registered ? this : (this.parent?.closest() ?? null);
	}
}

function track() {
	const events: string[] = [];
	const tracker = new TooltipTracker<FakeTarget>(
		(source) => events.push(`show ${source.name}`),
		() => events.push('hide')
	);
	return { tracker, events };
}

describe('TooltipTracker', () => {
	const canvas = new FakeTarget('canvas');
	const cardA = new FakeTarget('a', canvas, true);
	const cardB = new FakeTarget('b', canvas, true);
	const label = new FakeTarget('label', cardA);
	const memberRow = new FakeTarget('member', cardA, true);

	it('moves straight from one card to the next without closing in between', () => {
		const { tracker, events } = track();
		tracker.moveTo(label);
		tracker.moveTo(cardB);
		expect(events).toEqual(['show a', 'show b']);
	});

	it('stays put while the pointer moves inside the same card', () => {
		const { tracker, events } = track();
		tracker.moveTo(cardA);
		tracker.moveTo(label);
		expect(events).toEqual(['show a']);
	});

	it('shows the innermost registered element, and closes on leaving every one', () => {
		const { tracker, events } = track();
		tracker.moveTo(memberRow);
		tracker.moveTo(label);
		tracker.moveTo(canvas);
		tracker.moveTo(null);
		expect(events).toEqual(['show member', 'show a', 'hide']);
	});

	it('keeps a tapped tooltip open after the touch ends, until a tap lands elsewhere', () => {
		const { tracker, events } = track();
		tracker.moveTo(cardA);
		tracker.leave(null, 'touch');
		tracker.moveTo(canvas);
		expect(events).toEqual(['show a', 'hide']);
	});

	it('closes when a mouse pointer leaves', () => {
		const { tracker, events } = track();
		tracker.moveTo(cardA);
		tracker.leave(null, 'mouse');
		expect(events).toEqual(['show a', 'hide']);
	});

	it('closes when the shown element is destroyed, and ignores others going away', () => {
		const { tracker, events } = track();
		tracker.moveTo(cardA);
		tracker.refresh(cardB, true);
		tracker.refresh(cardA, true);
		expect(events).toEqual(['show a', 'hide']);
		expect(tracker.current).toBeNull();
	});
});

describe('dismissing the tooltip', () => {
	it('closes it now, and shows again on the next element the pointer reaches', () => {
		const page = new FakeTarget('page');
		const chip = new FakeTarget('chip', page, true);
		const other = new FakeTarget('other', page, true);
		const { tracker, events } = track();
		tracker.moveTo(chip);
		tracker.dismiss();
		expect(events).toEqual(['show chip', 'hide']);
		expect(tracker.current).toBeNull();

		tracker.moveTo(other);
		expect(events).toEqual(['show chip', 'hide', 'show other']);
	});
});

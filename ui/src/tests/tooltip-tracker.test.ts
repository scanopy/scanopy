import { afterEach, beforeEach, describe, it, expect, vi } from 'vitest';
import { SURFACE_ATTRIBUTE, TooltipTracker, type TooltipTarget } from '$lib/shared/actions/tooltip';

/** An element in a tree, as far as the tracker reads one: whether it or an ancestor is registered. */
class FakeTarget implements TooltipTarget {
	constructor(
		readonly name: string,
		readonly parent: FakeTarget | null = null,
		readonly registered = false,
		readonly surface = false
	) {}
	closest(selector: string): FakeTarget | null {
		const matches = selector.includes(SURFACE_ATTRIBUTE) ? this.surface : this.registered;
		return matches ? this : (this.parent?.closest(selector) ?? null);
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

describe('a tooltip holding chips to click', () => {
	const page = new FakeTarget('page');
	const chip = new FakeTarget('chip', page, true);
	const other = new FakeTarget('other', page, true);
	const surface = new FakeTarget('surface', null, false, true);
	const nested = new FakeTarget('nested', surface, true);

	function trackReachable() {
		const events: string[] = [];
		const tracker = new TooltipTracker<FakeTarget>(
			(source) => events.push(`show ${source.name}`),
			() => events.push('hide'),
			() => true
		);
		return { tracker, events };
	}

	beforeEach(() => {
		vi.useFakeTimers();
	});
	afterEach(() => {
		vi.useRealTimers();
	});

	it('waits for the pointer to cross the gap, and stays once it arrives', () => {
		const { tracker, events } = trackReachable();
		tracker.moveTo(chip);
		tracker.moveTo(page);
		tracker.moveTo(surface);
		vi.runAllTimers();
		expect(events).toEqual(['show chip']);
	});

	it('closes after the grace when the pointer goes nowhere', () => {
		const { tracker, events } = trackReachable();
		tracker.moveTo(chip);
		tracker.moveTo(page);
		expect(events).toEqual(['show chip']);
		vi.runAllTimers();
		expect(events).toEqual(['show chip', 'hide']);
		expect(tracker.current).toBeNull();
	});

	it('gives a nested chip no tooltip of its own', () => {
		const { tracker, events } = trackReachable();
		tracker.moveTo(chip);
		tracker.moveTo(nested);
		vi.runAllTimers();
		expect(events).toEqual(['show chip']);
	});

	it('switches to another source at once', () => {
		const { tracker, events } = trackReachable();
		tracker.moveTo(chip);
		tracker.moveTo(page);
		tracker.moveTo(other);
		vi.runAllTimers();
		expect(events).toEqual(['show chip', 'show other']);
	});

	it('closes on leaving the tooltip, and at once on a chip click', () => {
		const { tracker, events } = trackReachable();
		tracker.moveTo(chip);
		tracker.moveTo(surface);
		tracker.moveTo(page);
		vi.runAllTimers();
		expect(events).toEqual(['show chip', 'hide']);

		tracker.moveTo(chip);
		tracker.dismiss();
		expect(events).toEqual(['show chip', 'hide', 'show chip', 'hide']);
	});

	it('closes as it always did when there is nothing in it to reach', () => {
		const { tracker, events } = track();
		tracker.moveTo(chip);
		tracker.moveTo(surface);
		expect(events).toEqual(['show chip', 'hide']);
	});
});

import { describe, expect, it } from 'vitest';
import {
	fitToViewport,
	MIN_POPOVER_HEIGHT_PX,
	TRIGGER_GAP_PX,
	VIEWPORT_MARGIN_PX
} from '#lib/shared/utils/popoverPosition.ts';

const viewport = { width: 1000, height: 800 };

/** A 20px-tall trigger whose top edge sits at `top`. */
function triggerAt(top: number, left = 400, width = 100) {
	return { top, bottom: top + 20, left, width };
}

describe('fitToViewport', () => {
	it('opens below and caps at the margin from the bottom edge', () => {
		const trigger = triggerAt(100);
		const fit = fitToViewport({ trigger, contentHeight: 300, width: 200, viewport });

		expect(fit.top).toBe(trigger.bottom + TRIGGER_GAP_PX);
		expect(fit.bottom).toBeNull();
		// Grown to its cap, the popover's bottom edge sits exactly the margin above the viewport's.
		expect(fit.top! + fit.maxHeight).toBe(viewport.height - VIEWPORT_MARGIN_PX);
	});

	it('flips above when the content overflows below and above has more room', () => {
		const trigger = triggerAt(600);
		const fit = fitToViewport({ trigger, contentHeight: 300, width: 200, viewport });

		expect(fit.top).toBeNull();
		expect(fit.bottom).toBe(viewport.height - trigger.top + TRIGGER_GAP_PX);
		// Its top edge, at the cap, sits the margin below the viewport's top.
		expect(viewport.height - fit.bottom! - fit.maxHeight).toBe(VIEWPORT_MARGIN_PX);
	});

	it('stays below when the content fits there, however much room is above', () => {
		const fit = fitToViewport({
			trigger: triggerAt(600),
			contentHeight: 100,
			width: 200,
			viewport
		});
		expect(fit.top).not.toBeNull();
	});

	it('stays below when the content overflows both sides but below has more room', () => {
		const fit = fitToViewport({
			trigger: triggerAt(300),
			contentHeight: 2000,
			width: 200,
			viewport
		});
		expect(fit.top).not.toBeNull();
	});

	it('prefers above when asked, and flips below at the top edge', () => {
		const above = fitToViewport({
			trigger: triggerAt(400),
			contentHeight: 50,
			width: 200,
			viewport,
			prefer: 'above'
		});
		expect(above.bottom).not.toBeNull();

		const nearTop = fitToViewport({
			trigger: triggerAt(30),
			contentHeight: 50,
			width: 200,
			viewport,
			prefer: 'above'
		});
		expect(nearTop.top).not.toBeNull();
	});

	it('never caps the height below the floor', () => {
		const fit = fitToViewport({
			// 120px of room on each side of the trigger.
			trigger: triggerAt(140),
			contentHeight: 2000,
			width: 200,
			viewport: { width: 1000, height: 300 }
		});
		expect(fit.maxHeight).toBe(MIN_POPOVER_HEIGHT_PX);
	});

	it('centres on the trigger and clamps to the side margins', () => {
		const centred = fitToViewport({
			trigger: triggerAt(100),
			contentHeight: 50,
			width: 200,
			viewport
		});
		expect(centred.left + 100).toBe(450);

		const atLeft = fitToViewport({
			trigger: triggerAt(100, 0, 20),
			contentHeight: 50,
			width: 200,
			viewport
		});
		expect(atLeft.left).toBe(VIEWPORT_MARGIN_PX);

		const atRight = fitToViewport({
			trigger: triggerAt(100, 980, 20),
			contentHeight: 50,
			width: 200,
			viewport,
			align: 'start'
		});
		expect(atRight.left + 200).toBe(viewport.width - VIEWPORT_MARGIN_PX);
	});

	it('lines up with the trigger when aligned to start', () => {
		const fit = fitToViewport({
			trigger: triggerAt(100, 300),
			contentHeight: 50,
			width: 200,
			viewport,
			align: 'start'
		});
		expect(fit.left).toBe(300);
	});
});

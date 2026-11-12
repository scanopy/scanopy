import { describe, it, expect, vi } from 'vitest';
import { get } from 'svelte/store';
import {
	assignModalKeys,
	closesModalOnEscape,
	isStackedModal,
	isTopmostModal,
	openModalStack,
	keyPausedAt,
	registerOpenModal,
	resolveModalKeydown,
	ENTER_KEY,
	ESCAPE_KEY,
	type ModalAction,
	type ModalKeydown
} from '#lib/shared/components/layout/modalActions.ts';
import { DELETE_KEY } from '#lib/shared/components/data/controls/actionShortcuts.ts';

const action = (role: ModalAction['role'], label: string, extra: Partial<ModalAction> = {}) =>
	({ role, label, onClick: vi.fn(), ...extra }) as ModalAction;

/** A stand-in for the element a key landed on, with what the resolver reads from it. */
const element = (tagName: string, extra: Record<string, unknown> = {}) =>
	({
		tagName,
		type: 'text',
		isContentEditable: false,
		getAttribute: (name: string) => (extra.attributes as Record<string, string>)?.[name] ?? null,
		...extra
	}) as unknown as EventTarget;

const keydown = (key: string, target: EventTarget, extra: Partial<ModalKeydown> = {}) =>
	({
		key,
		target,
		metaKey: false,
		ctrlKey: false,
		altKey: false,
		shiftKey: false,
		repeat: false,
		isComposing: false,
		defaultPrevented: false,
		...extra
	}) as ModalKeydown;

const textInput = element('INPUT');
const textarea = element('TEXTAREA');

/**
 * A form modal's actions, as an editor declares them: the primary action submits through
 * validation, so it reaches `handleSubmit` only when the form is valid.
 */
function formModal(valid: boolean, disabled = false) {
	const handleSubmit = vi.fn();
	const submitForm = async () => {
		if (valid) handleSubmit();
	};
	const keyed = assignModalKeys([
		action('cancel', 'Cancel'),
		action('primary', 'Save', { onClick: submitForm, disabled })
	]);
	return { keyed, handleSubmit };
}

/** Runs the action a keydown resolves to, as GenericModal does. */
async function press(event: ModalKeydown, keyed: ReturnType<typeof assignModalKeys>) {
	const result = resolveModalKeydown(event, keyed);
	await result.action?.onClick();
	return result;
}

describe('Enter on a form modal', () => {
	it('submits a valid form from a text field, as the button does', async () => {
		const { keyed, handleSubmit } = formModal(true);
		const result = await press(keydown('Enter', textInput), keyed);
		expect(result.preventDefault).toBe(true);
		expect(handleSubmit).toHaveBeenCalledTimes(1);
	});

	it('stops at validation on an invalid form, as a click does', async () => {
		const { keyed, handleSubmit } = formModal(false);
		await press(keydown('Enter', textInput), keyed);
		await keyed.find(({ key }) => key === ENTER_KEY)!.action.onClick();
		expect(handleSubmit).not.toHaveBeenCalled();
	});

	it('does nothing while the primary action is disabled, and still blocks implicit submit', async () => {
		const { keyed, handleSubmit } = formModal(true, true);
		const result = await press(keydown('Enter', textInput), keyed);
		expect(result).toEqual({ action: null, preventDefault: true });
		expect(handleSubmit).not.toHaveBeenCalled();
	});

	it('ignores a held key, so holding Enter submits once', () => {
		const { keyed } = formModal(true);
		expect(resolveModalKeydown(keydown('Enter', textInput, { repeat: true }), keyed).action).toBe(
			null
		);
	});
});

describe('Enter in a textarea', () => {
	const keyed = assignModalKeys([action('primary', 'Save')]);

	it('types a newline', () => {
		expect(resolveModalKeydown(keydown('Enter', textarea), keyed)).toEqual({
			action: null,
			preventDefault: false
		});
	});

	it.each([{ metaKey: true }, { ctrlKey: true }])('runs the primary action with %o', (mod) => {
		expect(resolveModalKeydown(keydown('Enter', textarea, mod), keyed).action?.label).toBe('Save');
	});
});

describe('Enter that belongs to a control', () => {
	const keyed = assignModalKeys([action('primary', 'Save')]);

	it('leaves an open combobox to pick its option', () => {
		const combobox = element('INPUT', { attributes: { 'aria-expanded': 'true' } });
		expect(resolveModalKeydown(keydown('Enter', combobox), keyed).action).toBe(null);
	});

	it('leaves a key a dropdown already handled', () => {
		const event = keydown('Enter', textInput, { defaultPrevented: true });
		expect(resolveModalKeydown(event, keyed).action).toBe(null);
	});

	it('leaves a focused button to press itself', () => {
		expect(resolveModalKeydown(keydown('Enter', element('BUTTON')), keyed).action).toBe(null);
	});

	// The daemon wizard: clicking an OS or a step leaves focus on that choice, and Enter re-picked
	// it instead of moving on.
	it('runs the primary action from a focused choice button', () => {
		for (const attributes of [
			{ 'aria-pressed': 'true' },
			{ 'aria-current': 'step' },
			{ role: 'tab' }
		]) {
			const choice = element('BUTTON', { attributes });
			expect(resolveModalKeydown(keydown('Enter', choice), keyed).action?.label).toBe('Save');
		}
	});

	it('runs the primary action with a closed combobox focused', () => {
		const combobox = element('INPUT', { attributes: { 'aria-expanded': 'false' } });
		expect(resolveModalKeydown(keydown('Enter', combobox), keyed).action?.label).toBe('Save');
	});
});

describe('keys for the other actions', () => {
	const keyed = assignModalKeys([
		action('danger', 'Delete'),
		action('secondary', 'Back'),
		action('cancel', 'Cancel'),
		action('primary', 'Save')
	]);
	const body = element('BODY');

	it('gives cancel Escape, danger Delete, and secondaries a letter', () => {
		expect(keyed.map(({ key }) => key)).toEqual([DELETE_KEY, 'B', ESCAPE_KEY, ENTER_KEY]);
	});

	it('runs a secondary on its letter outside a field', () => {
		expect(resolveModalKeydown(keydown('b', body), keyed).action?.label).toBe('Back');
	});

	it('never fires a letter while the user types in a field', () => {
		expect(resolveModalKeydown(keydown('b', textInput), keyed).action).toBe(null);
	});

	it('leaves bare Delete and Backspace to a field that edits text', () => {
		const contentEditable = element('DIV', { isContentEditable: true });
		for (const target of [textInput, textarea, element('SELECT'), contentEditable]) {
			for (const key of ['Delete', 'Backspace']) {
				expect(resolveModalKeydown(keydown(key, target), keyed)).toEqual({
					action: null,
					preventDefault: false
				});
			}
		}
	});

	it('runs danger on bare Delete or Backspace, not with a modifier held', () => {
		expect(resolveModalKeydown(keydown('Delete', body), keyed).action?.label).toBe('Delete');
		expect(resolveModalKeydown(keydown('Backspace', body), keyed).action?.label).toBe('Delete');
		expect(resolveModalKeydown(keydown('Delete', body, { metaKey: true }), keyed).action).toBe(
			null
		);
		expect(resolveModalKeydown(keydown('Delete', body, { ctrlKey: true }), keyed).action).toBe(
			null
		);
	});

	it('gives a second danger action no key rather than a bare letter', () => {
		const twoDangers = assignModalKeys([action('danger', 'Delete'), action('danger', 'Purge')]);
		expect(twoDangers.map(({ key }) => key)).toEqual([DELETE_KEY, null]);
		expect(resolveModalKeydown(keydown('p', body), twoDangers).action).toBe(null);
	});

	it('skips a disabled action', () => {
		const disabled = assignModalKeys([action('secondary', 'Back', { disabled: true })]);
		expect(resolveModalKeydown(keydown('b', body), disabled)).toEqual({
			action: null,
			preventDefault: false
		});
	});
});

describe('stacked modals', () => {
	it('lets only the topmost modal answer, handing back to the one beneath on close', () => {
		const editor = Symbol('editor');
		const confirm = Symbol('confirm');
		const closeEditor = registerOpenModal(editor);
		const closeConfirm = registerOpenModal(confirm);
		expect(isTopmostModal(confirm)).toBe(true);
		expect(isTopmostModal(editor)).toBe(false);
		closeConfirm();
		expect(isTopmostModal(editor)).toBe(true);
		closeEditor();
		expect(isTopmostModal(editor)).toBe(false);
	});

	it('shrouds a modal over another only once the one beneath closes', () => {
		const settings = Symbol('settings');
		const payment = Symbol('payment');
		const closeSettings = registerOpenModal(settings);
		const closePayment = registerOpenModal(payment);
		expect(isStackedModal(get(openModalStack), payment)).toBe(true);
		expect(isStackedModal(get(openModalStack), settings)).toBe(false);
		closeSettings();
		expect(isStackedModal(get(openModalStack), payment)).toBe(false);
		closePayment();
	});

	it('shrouds a modal over one that darkens only its own area', () => {
		const prompt = Symbol('anchored prompt');
		const plans = Symbol('plans');
		const closePrompt = registerOpenModal(prompt, false);
		const closePlans = registerOpenModal(plans);
		expect(isStackedModal(get(openModalStack), plans)).toBe(false);
		closePlans();
		closePrompt();
	});
});

describe('chips that dim while typing', () => {
	// Every chip's key, checked against whether pressing it on that target runs its action.
	const keys = [ENTER_KEY, ESCAPE_KEY, 'B', DELETE_KEY];
	const keyed = assignModalKeys([
		action('cancel', 'Cancel'),
		action('secondary', 'Back'),
		action('danger', 'Delete'),
		action('primary', 'Save')
	]);
	const fires = (key: string, target: EventTarget) => {
		return resolveModalKeydown(keydown(key, target), keyed).action !== null;
	};

	it('dims Enter and the letters in a textarea, where typing goes into the field', () => {
		expect(keys.filter((k) => keyPausedAt(k, textarea))).toEqual([ENTER_KEY, 'B', DELETE_KEY]);
	});

	it('dims the letters but not Enter in a text input, which Enter submits from', () => {
		expect(keys.filter((k) => keyPausedAt(k, textInput))).toEqual(['B', DELETE_KEY]);
	});

	it('dims nothing on a focused button', () => {
		expect(keys.filter((k) => keyPausedAt(k, element('BUTTON')))).toEqual([]);
	});

	it('dims exactly the keys that do nothing on a field', () => {
		for (const target of [textarea, textInput]) {
			for (const key of keys.filter((k) => k !== ESCAPE_KEY)) {
				expect(keyPausedAt(key, target)).toBe(!fires(key, target));
			}
		}
	});
});

describe('a tag picker open in a modal', () => {
	// The picker's search input, with its dropdown showing.
	const search = element('INPUT', { attributes: { 'aria-expanded': 'true' } });
	const keyed = assignModalKeys([
		action('danger', 'Delete'),
		action('secondary', 'Back'),
		action('cancel', 'Cancel'),
		action('primary', 'Save')
	]);

	it('leaves Enter to pick a tag, and dims the Enter chip', () => {
		expect(resolveModalKeydown(keydown('Enter', search), keyed).action).toBe(null);
		expect(keyPausedAt(ENTER_KEY, search)).toBe(true);
	});

	it('leaves Backspace and Delete to the search, never running danger', () => {
		for (const key of ['Backspace', 'Delete']) {
			expect(resolveModalKeydown(keydown(key, search), keyed).action).toBe(null);
		}
	});

	it('leaves letters to type into the search', () => {
		expect(resolveModalKeydown(keydown('b', search), keyed).action).toBe(null);
	});

	it('lets Escape close the picker without closing the modal', () => {
		expect(closesModalOnEscape(keydown('Escape', search, { defaultPrevented: true }))).toBe(false);
		expect(closesModalOnEscape(keydown('Escape', textInput))).toBe(true);
	});
});

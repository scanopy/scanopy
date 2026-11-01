/**
 * A modal's footer actions, declared as data, and the keys that run them.
 *
 * The primary action takes Enter (and Mod+Enter, which also works from a textarea). Cancel shows
 * Escape, which the modal already answers by closing. A danger action takes Delete or Backspace,
 * with no modifier. Every other action takes a letter from `assignActionKeys`, as the list
 * toolbar's actions do. Only the topmost open modal answers keys.
 */
import type { ActionTone } from '$lib/shared/components/data/types';
import type { IconComponent } from '$lib/shared/utils/types';
import { assignActionKeys, DELETE_KEY } from '$lib/shared/components/data/controls/actionShortcuts';
import { isEditableTarget } from '$lib/shared/utils/shortcuts';

export type ModalActionRole = 'primary' | 'secondary' | 'danger' | 'cancel';

export interface ModalAction {
	role: ModalActionRole;
	label: string;
	onClick: () => unknown;
	disabled?: boolean;
	/** Overrides the role's colour: a confirm dialog's danger variant tones its primary action. */
	tone?: ActionTone;
	/** Shown on hover: the hint, or why the action is disabled. */
	tooltip?: string;
	/** The letter this action prefers, when free. */
	shortcut?: string;
	/** Which end of the footer the button sits at. Danger actions default to the start. */
	align?: 'start' | 'end';
	/** Drawn before the label. Nothing goes after it: the key hint sits there. */
	icon?: IconComponent;
	/** The action is running: a spinner replaces the icon. */
	busy?: boolean;
	/** The larger primary button wizards use for Next. */
	large?: boolean;
}

/** The key that runs the primary action. */
export const ENTER_KEY = 'Enter';
/** The key the cancel action shows; the modal closes on it. */
export const ESCAPE_KEY = 'Escape';

export interface KeyedModalAction {
	action: ModalAction;
	/** `ENTER_KEY`, `ESCAPE_KEY`, `DELETE_KEY`, a capital letter, or null. */
	key: string | null;
}

/**
 * Each action's key. The first primary takes Enter and the first cancel Escape. The first danger
 * action takes Delete; any other danger action goes without a key rather than take a letter.
 * Secondary actions take letters as the toolbar's do.
 */
export function assignModalKeys(actions: ModalAction[]): KeyedModalAction[] {
	const primary = actions.find((a) => a.role === 'primary');
	const cancel = actions.find((a) => a.role === 'cancel');
	const danger = actions.find((a) => a.role === 'danger');
	const lettered = actions.filter((a) => a.role === 'secondary' || a === danger);
	const letters = new Map(
		assignActionKeys(
			lettered.map((a) => ({ ...a, isDelete: a === danger, source: a })),
			[]
		).map(({ action, key }) => [action.source, key])
	);
	return actions.map((action) => {
		if (action === primary) return { action, key: ENTER_KEY };
		if (action === cancel) return { action, key: ESCAPE_KEY };
		if (action.role === 'secondary' || action === danger) {
			return { action, key: letters.get(action) ?? null };
		}
		return { action, key: null };
	});
}

/**
 * Whether `key` does nothing while focus sits on `target`, so its chip dims. Letters and Delete
 * pause in any text field, where they type or erase. Enter pauses where the field keeps it (a
 * textarea types a newline), and Mod+Enter runs the action there instead. Escape never pauses.
 */
export function keyPausedAt(key: string, target: EventTarget | null): boolean {
	if (key === ESCAPE_KEY || !isEditableTarget(target)) return false;
	return key === ENTER_KEY ? !takesEnterForPrimary(target) : true;
}

/** The parts of a keydown the resolver reads, so tests can pass plain objects. */
export type ModalKeydown = Pick<
	KeyboardEvent,
	| 'key'
	| 'metaKey'
	| 'ctrlKey'
	| 'altKey'
	| 'shiftKey'
	| 'repeat'
	| 'isComposing'
	| 'defaultPrevented'
	| 'target'
>;

export interface ModalKeydownResult {
	/** The action to run, or null. */
	action: ModalAction | null;
	/** Whether to stop the key's own effect, such as a form's implicit submission. */
	preventDefault: boolean;
}

const NOTHING: ModalKeydownResult = { action: null, preventDefault: false };

/**
 * What a keydown inside the topmost modal does to its footer actions.
 *
 * Enter runs the primary action from a text input, a checkbox or the panel itself. It leaves alone
 * a textarea or contenteditable (Enter types a newline there; Mod+Enter runs the primary action
 * instead), a select or an open combobox (Enter picks the option), and a focused button or link
 * (Enter presses it), except a choice button such as an OS toggle or a step, where it runs the
 * primary action. In a text input Enter is stopped even while the primary action is disabled,
 * so the browser's implicit form submission can't run what the button won't.
 *
 * Letters and Delete never fire while focus is in a text field, where those keys edit text.
 */
export function resolveModalKeydown(
	event: ModalKeydown,
	keyed: KeyedModalAction[]
): ModalKeydownResult {
	if (event.defaultPrevented || event.isComposing || event.repeat) return NOTHING;
	const mod = event.metaKey || event.ctrlKey;
	const editable = isEditableTarget(event.target);

	if (event.key === 'Enter') {
		if (event.altKey || event.shiftKey) return NOTHING;
		const primary = keyed.find(({ key }) => key === ENTER_KEY)?.action;
		if (!mod && !takesEnterForPrimary(event.target)) return NOTHING;
		if (!primary) return NOTHING;
		return { action: primary.disabled ? null : primary, preventDefault: true };
	}

	if (editable || event.altKey || event.shiftKey) return NOTHING;
	const match = keyed.find(({ key }) => {
		if (key === null || key === ENTER_KEY || key === ESCAPE_KEY) return false;
		if (key === DELETE_KEY) return !mod && (event.key === 'Delete' || event.key === 'Backspace');
		return !mod && event.key.toUpperCase() === key;
	});
	if (!match || match.action.disabled) return NOTHING;
	return { action: match.action, preventDefault: true };
}

/**
 * Whether a keydown closes the topmost modal: Escape, unless a control inside it (an open picker
 * or dropdown) already answered it by closing itself.
 */
export function closesModalOnEscape(
	event: Pick<KeyboardEvent, 'key' | 'defaultPrevented' | 'isComposing'>
): boolean {
	return event.key === ESCAPE_KEY && !event.defaultPrevented && !event.isComposing;
}

/** Whether plain Enter on this target means the primary action rather than its own job. */
function takesEnterForPrimary(target: EventTarget | null): boolean {
	const el = target as HTMLElement | null;
	if (!el || typeof el.tagName !== 'string') return true;
	if (el.isContentEditable) return false;
	if (el.tagName === 'BUTTON' && isChoiceButton(el)) return true;
	if (['TEXTAREA', 'SELECT', 'BUTTON', 'A', 'SUMMARY'].includes(el.tagName)) return false;
	if (el.getAttribute?.('aria-expanded') === 'true') return false;
	return true;
}

/**
 * A button that picks one of several options (an OS, a mode, a step) rather than doing something.
 * Pressing it again changes nothing, so Enter on it runs the primary action, as Enter in a native
 * radio group submits the form. Space still presses it.
 */
function isChoiceButton(el: HTMLElement): boolean {
	const has = (name: string) => el.getAttribute?.(name) != null;
	const role = el.getAttribute?.('role');
	return (
		has('aria-pressed') ||
		has('aria-current') ||
		has('aria-selected') ||
		role === 'radio' ||
		role === 'tab' ||
		role === 'switch'
	);
}

/** Open modals, oldest first. Only the last one answers keys shared by every modal. */
const openModals: symbol[] = [];

/**
 * Records `token`'s modal as opened on top; the returned function records it closed.
 *
 * The page stops scrolling while any modal is open, and scrolls again only when the last one
 * closes, so closing a dialog stacked over another leaves the one beneath it locked.
 */
export function registerOpenModal(token: symbol): () => void {
	openModals.push(token);
	if (typeof document !== 'undefined') document.body.style.overflow = 'hidden';
	return () => {
		const at = openModals.indexOf(token);
		if (at !== -1) openModals.splice(at, 1);
		if (openModals.length === 0 && typeof document !== 'undefined') {
			document.body.style.overflow = '';
		}
	};
}

/** Whether any modal is open, for a modal about to open over it. */
export function isAnyModalOpen(): boolean {
	return openModals.length > 0;
}

export function isTopmostModal(token: symbol): boolean {
	return openModals.at(-1) === token;
}

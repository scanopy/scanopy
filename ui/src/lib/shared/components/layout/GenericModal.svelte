<script lang="ts" module>
	import type { IconComponent } from '$lib/shared/utils/types';

	/**
	 * Tab definition for modal header tabs
	 */
	export interface ModalTab {
		id: string;
		label: string;
		icon?: IconComponent;
		notification?: boolean;
		/** A count shown beside the label, as an amber chip. Hidden at 0. */
		count?: number;
		disabled?: boolean;
	}
</script>

<script lang="ts">
	import { tick, untrack, type Snippet } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { ArrowLeft, Loader2, X } from '@lucide/svelte';
	import type { AnyFormApi } from '@tanstack/form-core';
	import {
		common_closeModal,
		common_discard,
		common_discardChangesMessage,
		common_discardChangesTitle,
		common_keepEditing,
		common_modal,
		common_modalTabs,
		common_next,
		common_previous,
		common_shortcutsPausedWhileTyping,
		common_shortcutWhileTyping
	} from '$lib/paraglide/messages';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import { isEditableTarget, keyLabel, shortcutLabel } from '$lib/shared/utils/shortcuts';
	import ModalStepper from './ModalStepper.svelte';
	import ServerUnreachableBanner from '$lib/shared/components/feedback/ServerUnreachableBanner.svelte';
	import { serverUnreachable } from '$lib/api/server-reachability';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { toolbarButtonClass } from '$lib/shared/components/data/controls/actionShortcuts';
	import {
		assignModalKeys,
		closesModalOnEscape,
		ENTER_KEY,
		isAnyModalOpen,
		isTopmostModal,
		keyPausedAt,
		registerOpenModal,
		resolveModalKeydown,
		type ModalAction
	} from './modalActions';
	import { provideModalActions } from './modalActionsContext.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { toColor } from '$lib/shared/utils/styling';
	import { get } from 'svelte/store';
	import {
		modalState,
		openModal,
		closeModal,
		adjacentEntityId,
		entityListOrderFor,
		restoreModal,
		setModalTab,
		goBack
	} from '$lib/shared/stores/modal-registry';

	let {
		title = common_modal(),
		centerTitle = false,
		isOpen = false,
		keepMounted = false,
		onClose = null,
		size = 'lg',
		anchor = 'viewport',
		preventCloseOnClickOutside = false,
		showCloseButton = true,
		showTitleRow = true,
		showBackdrop = true,
		borderless = false,
		floatingCloseButton = false,
		fixedHeight = false,
		compactPadding = false,
		tabs = [],
		activeTab = $bindable(''),
		tabStyle = 'tabs',
		onTabChange = null,
		onOpen = null,
		onSubEntityNavigation = null,
		instanceKey = $bindable(0),
		name = undefined,
		entityId = undefined,
		form = undefined,
		unsavedState = undefined,
		headerIcon,
		banners,
		noBanners = false,
		children,
		footer,
		actions = [],
		footerLayout = 'row',
		footerNote
	}: {
		title?: string;
		centerTitle?: boolean;
		isOpen?: boolean;
		/**
		 * Render the modal's content while closed, hidden and inert, so what it loads (an
		 * iframe) is ready the moment it opens. Every open/close effect still keys on `isOpen`.
		 */
		keepMounted?: boolean;
		onClose?: (() => void) | null;
		size?: 'sm' | 'md' | 'lg' | 'xl' | 'wide' | 'full' | 'max';
		/**
		 * What the modal is positioned against. `viewport` covers the whole window; `container`
		 * fills the nearest positioned ancestor instead, for a modal that belongs to one region of
		 * the page rather than to the app.
		 */
		anchor?: 'viewport' | 'container';
		preventCloseOnClickOutside?: boolean;
		showCloseButton?: boolean;
		/**
		 * False drops the title row, leaving the title for screen readers only: the search palette,
		 * and Settings while it gates the app (its banners take the row's place).
		 */
		showTitleRow?: boolean;
		showBackdrop?: boolean;
		borderless?: boolean;
		floatingCloseButton?: boolean;
		fixedHeight?: boolean;
		compactPadding?: boolean;
		tabs?: ModalTab[];
		activeTab?: string;
		tabStyle?: 'tabs' | 'stepper';
		onTabChange?: ((tabId: string) => void) | null;
		onOpen?: (() => void) | null;
		onSubEntityNavigation?: ((subEntityId: string) => void) | null;
		instanceKey?: number;
		name?: string;
		entityId?: string;
		/**
		 * The editor's form. Arrow-key navigation to the previous or next entity asks before
		 * leaving when it holds unsaved changes.
		 */
		form?: AnyFormApi;
		/** Editable state the editor keeps outside `form`, such as a host's interface list. */
		unsavedState?: () => unknown;
		headerIcon?: Snippet;
		/**
		 * Rendered inside the panel frame, above the title. Opt-in: this component
		 * backs the login, register and share-password modals too, and app banners
		 * have no business over those. Only a modal that gates the app behind it
		 * passes this.
		 */
		banners?: Snippet;
		/**
		 * Shows no banner at all, the server-update banner included, which otherwise shows over
		 * every modal. The tab still reloads once the modal closes.
		 */
		noBanners?: boolean;
		children?: Snippet<[number]>;
		/** A hand-built footer, for one the actions below can't describe. */
		footer?: Snippet;
		/**
		 * The footer's buttons. The modal draws them, toned by role, and gives each its key: Enter
		 * for the primary action, Escape for cancel, Mod+Delete for danger, a letter for the rest.
		 * A component inside the modal can declare them instead, with `useModalActions`.
		 */
		actions?: ModalAction[];
		/**
		 * `stack` spans the button row across the footer and widens the primary action to fill it,
		 * for the sign-in modals' single call to action.
		 */
		footerLayout?: 'row' | 'stack';
		/** Lines under the footer's buttons, such as a link to sign in instead. */
		footerNote?: Snippet;
	} = $props();

	// Actions a component inside the modal declared, which take the place of the prop's.
	let declaredActions = $state<(() => ModalAction[]) | null>(null);
	provideModalActions({
		declare: (getActions) => {
			declaredActions = getActions;
			return () => {
				if (declaredActions === getActions) declaredActions = null;
			};
		}
	});
	let keyedActions = $derived(assignModalKeys(declaredActions ? declaredActions() : actions));
	let startActions = $derived(
		keyedActions.filter(({ action }) => (action.align ?? defaultAlign(action)) === 'start')
	);
	let endActions = $derived(
		keyedActions.filter(({ action }) => (action.align ?? defaultAlign(action)) === 'end')
	);

	function defaultAlign(action: ModalAction): 'start' | 'end' {
		return action.role === 'danger' ? 'start' : 'end';
	}

	const ROLE_CLASS: Record<ModalAction['role'], string> = {
		primary: 'btn-primary',
		secondary: 'btn-secondary',
		cancel: 'btn-secondary',
		danger: 'btn-danger'
	};

	function actionClass(action: ModalAction): string {
		const base = action.tone ? toolbarButtonClass(action.tone) : ROLE_CLASS[action.role];
		return action.large && action.role === 'primary' ? `${base} btn-primary-lg` : base;
	}

	// Keys pause while focus is in a field that takes them; their chips dim to match.
	let focusTarget = $state.raw<EventTarget | null>(null);
	function trackFocus(event: FocusEvent) {
		focusTarget = event.type === 'focusin' ? event.target : null;
	}

	// Tabs and steppers share the title row, between the title and the close button, whenever
	// that row shows a left-aligned title and the viewport is `md` or wider. Otherwise they keep
	// their own row below, which scrolls sideways when the tabs outgrow it. In the title row,
	// tabs never shrink below their full width; a long title truncates instead.
	const wideViewport = new MediaQuery('min-width: 768px');
	let inlineTabs = $derived(
		tabs.length > 0 && showTitleRow && !centerTitle && wideViewport.current
	);

	// Keep the active tab in view in a tab row that scrolls, including after a deep-link open.
	// Sets the row's own scroll position: `scrollIntoView` would scroll the modal page too.
	let tabRow = $state<HTMLElement | null>(null);
	$effect(() => {
		if (!isOpen || !tabRow || !activeTab) return;
		const row = tabRow;
		const current = row.querySelector<HTMLElement>('[aria-current]');
		if (!current || row.scrollWidth <= row.clientWidth) return;
		row.scrollLeft = current.offsetLeft - (row.clientWidth - current.offsetWidth) / 2;
	});

	let showBackButton = $derived(
		name != null && $modalState.name === name && $modalState.returnUrl != null
	);
	let returnTitle = $derived(
		name != null && $modalState.name === name ? $modalState.returnTitle : null
	);

	// Track previous open state to detect open transition
	let wasOpen = $state(false);

	// Stacking order: a dialog opened over this one takes Escape and the arrow keys, not both.
	// A modal opened over another leaves the darkening to the one beneath, so stacking doesn't
	// shroud the page twice.
	const stackToken = Symbol();
	let stacked = $state(false);
	$effect(() => {
		if (!isOpen) return;
		stacked = isAnyModalOpen();
		return registerOpenModal(stackToken);
	});

	function isTopmost(): boolean {
		return isTopmostModal(stackToken);
	}

	let panel = $state<HTMLElement | null>(null);

	// A modal opened over another takes focus, or the keys typed next land in the one beneath, which
	// no longer answers them. Waits a tick so a field that focuses itself on mount keeps focus.
	// Closing hands focus back to where it was.
	$effect(() => {
		if (!isOpen) return;
		const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
		tick().then(() => {
			if (panel && !panel.contains(document.activeElement)) panel.focus({ preventScroll: true });
		});
		return () => {
			if (previous?.isConnected) previous.focus({ preventScroll: true });
		};
	});

	// Left and Right step to the previous and next entity of the on-screen list this modal's
	// entity belongs to. The ends stop rather than wrap.
	let listOrder = $derived(isOpen && name && entityId ? entityListOrderFor(name, entityId) : null);
	let previousId = $derived(entityId ? adjacentEntityId(listOrder, entityId, -1) : null);
	let nextId = $derived(entityId ? adjacentEntityId(listOrder, entityId, 1) : null);

	/** The entity a step is waiting to reach while the discard-changes dialog is up. */
	let pendingNavigationId = $state<string | null>(null);
	/** The entity a step has asked the registry for, until the parent hands it over. */
	let navigatingToId: string | null = null;

	/**
	 * The editor's state as it stood when the user first touched it. Taken at the first pointer
	 * or key press inside the panel rather than at open, because editors keep settling after they
	 * open: fields normalise their values on mount and queries fill in data. Comparing against a
	 * snapshot from open flagged those as edits nobody made.
	 */
	let baseline: string | null = null;

	function editableSnapshot(): string {
		return JSON.stringify({
			form: form ? $state.snapshot(form.state.values) : null,
			other: unsavedState ? $state.snapshot(unsavedState()) : null
		});
	}

	function captureBaseline() {
		if (baseline === null && (form || unsavedState)) baseline = editableSnapshot();
	}

	function isDirty(): boolean {
		return baseline !== null && editableSnapshot() !== baseline;
	}

	function requestNavigation(targetId: string) {
		if (isDirty()) {
			pendingNavigationId = targetId;
		} else {
			navigateTo(targetId);
		}
	}

	function navigateTo(targetId: string) {
		if (!name) return;
		const state = get(modalState);
		navigatingToId = targetId;
		// The parent's deep-link effect answers the registry by handing this modal the new entity.
		openModal(name, {
			id: targetId,
			tab: activeTab || undefined,
			returnUrl: state.returnUrl ?? undefined,
			returnTitle: state.returnTitle ?? undefined
		});
	}

	function confirmPendingNavigation() {
		const targetId = pendingNavigationId;
		pendingNavigationId = null;
		if (targetId) navigateTo(targetId);
	}

	// Once the parent has swapped in the stepped-to entity, reload the editor as a fresh open
	// would, keeping the tab the user was on.
	$effect(() => {
		if (isOpen && entityId && entityId === navigatingToId) {
			navigatingToId = null;
			untrack(() => {
				const tab = activeTab;
				baseline = null;
				instanceKey++;
				onOpen?.();
				// onOpen resets the parent's tab; hand the user's tab back to it.
				if (tab && tabs.some((t) => t.id === tab)) {
					activeTab = tab;
					onTabChange?.(tab);
				}
			});
		}
	});

	function handleTabClick(tabId: string) {
		activeTab = tabId;
		if (name) {
			setModalTab(tabId);
		}
		onTabChange?.(tabId);
	}

	// Sync modal state with URL on open/close transitions
	$effect(() => {
		if (isOpen && !wasOpen) {
			baseline = null;
			instanceKey++;

			// Let the parent initialize first (e.g. reset form, set default tab)
			onOpen?.();

			// Check if modalState has a tab for this modal (from URL deep-link)
			const state = get(modalState);
			const urlTab = name && state.name === name ? state.tab : null;

			if (tabs.length > 0) {
				if (urlTab && tabs.some((t) => t.id === urlTab)) {
					// URL specifies a valid tab — override the default
					activeTab = urlTab;
					onTabChange?.(activeTab);
				} else if (!tabs.some((t) => t.id === activeTab)) {
					// Current activeTab is invalid — reset to first tab
					activeTab = tabs[0].id;
					onTabChange?.(activeTab);
				}
			}

			if (name) {
				// Read subEntityId, returnUrl, returnTitle and entityData before openModal
				// clears them. entityData is what the opener handed this modal (the plan a
				// payment dialog is collecting a card for); re-registering without it
				// dropped the plan the moment the dialog opened.
				const subEntityId = state.name === name ? state.subEntityId : null;
				const returnUrl = state.name === name ? (state.returnUrl ?? undefined) : undefined;
				const savedReturnTitle = state.name === name ? (state.returnTitle ?? undefined) : undefined;
				const entityData = state.name === name ? (state.entityData ?? undefined) : undefined;
				openModal(name, {
					id: entityId,
					tab: activeTab || undefined,
					returnUrl,
					returnTitle: savedReturnTitle,
					entityData
				});
				if (subEntityId && onSubEntityNavigation) {
					onSubEntityNavigation(subEntityId);
				}
			}
		} else if (!isOpen && wasOpen && name && get(modalState).name === name) {
			// Modal closed (by parent, form submit, etc.) — clear URL params
			// Only clear if the registry still refers to this modal (another modal may have opened)
			closeModal();
		}
		wasOpen = isOpen;
	});

	// Close this modal when the registry no longer points to it (modal-on-modal or goBack)
	$effect(() => {
		const state = $modalState;
		if (isOpen && name && state.name !== name) {
			// Capture whatever superseded this modal before handing control to the parent.
			//
			// A close handler that calls `closeModal()` unconditionally is the ordinary shape, and
			// it is right when the user dismissed this modal — but here the registry has already
			// moved on to another modal, and clearing it throws that one away. The symptom is a
			// chip or an action that navigates from inside one modal to another entity's editor
			// landing on the destination tab with nothing open, because the deep-link effect that
			// would have opened it found an empty registry. The branch above already applies this
			// rule to its own `closeModal()`; this one could not, because the clearing happens
			// inside the parent's handler.
			const superseding = state.name ? { ...state } : null;
			onClose?.();
			if (superseding && get(modalState).name === null) {
				restoreModal(superseding);
			}
		}
	});

	// Size classes
	const sizeClasses: Record<string, string> = {
		sm: 'max-w-md',
		md: 'max-w-lg',
		lg: 'max-w-2xl',
		xl: 'max-w-4xl',
		wide: 'max-w-6xl',
		full: 'max-w-7xl',
		max: 'max-w-none w-full'
	};

	// The panel's height, less the backdrop's padding. `dvh`, not `vh`: on iOS the browser's
	// toolbar covers the bottom of a `vh` box, and with it the footer. Below `sm` the back button
	// takes the top 4rem of the backdrop.
	let heightClass = $derived.by(() => {
		const fills = size === 'full' || size === 'wide' || fixedHeight;
		const phone = fills
			? showBackButton
				? 'h-[calc(100dvh-4.5rem)]'
				: 'h-[calc(100dvh-1rem)]'
			: showBackButton
				? 'max-h-[calc(100dvh-4.5rem)]'
				: 'max-h-[calc(100dvh-1rem)]';
		const desktop = fills
			? compactPadding
				? 'sm:h-[calc(100dvh-2rem)]'
				: 'sm:h-[calc(100dvh-8rem)]'
			: compactPadding
				? 'sm:max-h-[calc(100dvh-2rem)]'
				: 'sm:max-h-[calc(100dvh-8rem)]';
		return `${phone} ${desktop}`;
	});

	function handleClose() {
		activeTab = tabs.length > 0 ? tabs[0].id : '';
		if (name) {
			closeModal();
		}
		onClose?.();
	}

	function handleBackdropClick(event: MouseEvent) {
		if (!preventCloseOnClickOutside && event.target === event.currentTarget) {
			handleClose();
		}
	}

	// A key closes or acts on one modal only. Whichever listener answers it first marks it handled,
	// because closing the topmost modal can make the one beneath topmost before its listener runs.
	function handleKeydown(event: KeyboardEvent) {
		if (event.defaultPrevented || !isOpen || !isTopmost()) return;
		if (closesModalOnEscape(event)) {
			event.preventDefault();
			handleClose();
			return;
		}
		// Footer keys answer only within this panel, or with nothing focused: a popover portalled
		// to the body (an icon picker's search) keeps Enter for itself.
		const target = event.target;
		if (
			target === document.body ||
			(target instanceof Node && panel?.contains(target)) ||
			target === panel?.parentElement
		) {
			const { action, preventDefault } = resolveModalKeydown(event, keyedActions);
			if (preventDefault) event.preventDefault();
			if (action) {
				action.onClick();
				return;
			}
			if (preventDefault) return;
		}
		if (
			(event.key === 'ArrowLeft' || event.key === 'ArrowRight') &&
			!event.altKey &&
			!event.ctrlKey &&
			!event.metaKey &&
			!event.shiftKey &&
			!isEditableTarget(event.target)
		) {
			const targetId = event.key === 'ArrowLeft' ? previousId : nextId;
			if (!targetId) return;
			event.preventDefault();
			requestNavigation(targetId);
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen || keepMounted}
	<!-- Modal backdrop -->
	<div
		class="{showBackdrop && !stacked
			? 'modal-page modal-background'
			: 'modal-page'} {stacked ? 'p-4' : ''} {anchor === 'container'
			? 'modal-page-anchored'
			: ''} {compactPadding ? 'px-2! py-1! sm:px-4! sm:py-4!' : ''} {listOrder
			? 'sm:px-16!'
			: ''} {showBackButton ? 'max-sm:pt-16!' : ''} {isOpen ? '' : 'invisible'}"
		inert={!isOpen}
		aria-hidden={!isOpen}
		onclick={handleBackdropClick}
		role="dialog"
		aria-modal="true"
		aria-labelledby="modal-title"
		onkeydown={(e) => {
			if (!closesModalOnEscape(e) || !isTopmost()) return;
			e.preventDefault();
			handleClose();
		}}
		tabindex="-1"
	>
		<!-- Floating back button (upper-left of viewport, over backdrop) -->
		{#if showBackButton}
			<button
				type="button"
				onclick={() => goBack()}
				class="fixed top-6 left-6 z-50 flex items-center gap-2 rounded-full bg-white/80 py-2 pr-4 pl-2 text-sm text-gray-600 transition-colors hover:bg-gray-200 hover:text-gray-900 dark:bg-gray-800/80 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
				aria-label={returnTitle ? `Back to ${returnTitle}` : 'Go back'}
			>
				<ArrowLeft class="h-5 w-5" />
				<span class="max-w-48 truncate">{returnTitle ? `Back to ${returnTitle}` : 'Back'}</span>
			</button>
		{/if}

		<!-- Previous / next entity, over the backdrop either side of the panel. The backdrop's
		     side gutter (below) keeps the panel clear of them at every size. -->
		{#if listOrder}
			{@render entityStep('ArrowLeft', previousId, common_previous(), 'left-3')}
			{@render entityStep('ArrowRight', nextId, common_next(), 'right-3')}
		{/if}

		<!-- Modal content -->
		<div
			class="relative outline-hidden {borderless ? '' : 'modal-container'} {sizeClasses[
				size
			]} {heightClass} flex w-full flex-col"
			bind:this={panel}
			tabindex="-1"
			onpointerdowncapture={captureBaseline}
			onkeydowncapture={captureBaseline}
			onfocusin={trackFocus}
			onfocusout={trackFocus}
		>
			<!-- Floating close button (absolute positioned within modal container) -->
			{#if floatingCloseButton && onClose}
				<button
					type="button"
					onclick={handleClose}
					class="absolute top-3 right-4 z-30 rounded-full bg-white/80 p-2 text-gray-600 transition-colors hover:bg-gray-200 hover:text-gray-900 dark:bg-gray-800/80 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
					aria-label={common_closeModal()}
				>
					<X class="h-5 w-5" />
				</button>
			{/if}
			{#if !noBanners && (banners || $serverUnreachable)}
				<!-- Clipped here rather than by putting overflow-hidden on the panel,
				     which ~42 modals share: AppBanner is full-bleed with square corners
				     and the panel is rounded. The unreachable banner shows over every
				     modal, opted in or not: the page behind, which carries it otherwise,
				     is covered. -->
				<div class="shrink-0 overflow-hidden rounded-t-lg">
					<ServerUnreachableBanner />
					{@render banners?.()}
				</div>
			{/if}
			{#if !showTitleRow}
				<!-- Keeps the dialog's accessible name, which `aria-labelledby` points at, while the row is hidden. -->
				<h2 id="modal-title" class="sr-only">{title}</h2>
			{/if}
			<!-- Header (hidden when it would hold nothing: no title row and no tabs) -->
			{#if (showTitleRow && (title || showCloseButton)) || tabs.length > 0}
				<div class="modal-header flex-col gap-0 {tabs.length > 0 && !inlineTabs ? 'pb-0' : ''}">
					<!-- Title row -->
					{#if showTitleRow}
						<div class="flex w-full items-center justify-between {inlineTabs ? 'gap-6' : ''}">
							{#if centerTitle}
								{@render headerIcon?.()}
								<h2
									id="modal-title"
									class="absolute left-1/2 max-w-[calc(100%-5rem)] -translate-x-1/2 text-center text-xl font-semibold text-primary"
								>
									{title}
								</h2>
							{:else}
								<div class="flex min-w-0 items-center gap-3 {inlineTabs ? 'mr-4' : ''}">
									{@render headerIcon?.()}
									<h2 id="modal-title" class="truncate text-xl font-semibold text-primary">
										{title}
									</h2>
								</div>
							{/if}

							{#if inlineTabs}
								{@render tabNav(true)}
							{/if}

							{#if showCloseButton}
								<button
									type="button"
									onclick={handleClose}
									class="btn-icon"
									aria-label={common_closeModal()}
								>
									<X class="h-5 w-5" />
								</button>
							{/if}
						</div>
					{/if}

					<!-- Tab navigation on its own row, when the title row can't hold it -->
					{#if tabs.length > 0 && !inlineTabs}
						{@render tabNav(false)}
					{/if}
				</div>
			{/if}

			{#snippet tabNav(inline: boolean)}
				{#if tabStyle === 'stepper'}
					<ModalStepper {tabs} {activeTab} onTabClick={handleTabClick} {inline} />
				{:else}
					<!-- On its own row the tabs scroll sideways, snapping to each tab, once they
					     outgrow the panel (a phone). -->
					<nav
						bind:this={tabRow}
						class={inline
							? 'flex flex-1 items-center gap-6'
							: 'flex w-full snap-x snap-mandatory [scrollbar-width:none] space-x-6 overflow-x-auto pt-4'}
						aria-label={common_modalTabs()}
					>
						{#each tabs as tab (tab.id)}
							<button
								type="button"
								onclick={() => !tab.disabled && handleTabClick(tab.id)}
								class="shrink-0 snap-start border-b-2 px-1 text-sm font-medium whitespace-nowrap transition-colors
									{inline ? 'py-1' : 'pb-3'}
									{tab.disabled
									? 'cursor-not-allowed border-transparent text-muted opacity-50'
									: activeTab === tab.id
										? 'border-blue-500 text-primary'
										: 'border-transparent text-muted hover:text-secondary'}"
								aria-current={activeTab === tab.id ? 'page' : undefined}
								aria-disabled={tab.disabled ? 'true' : undefined}
							>
								<div class="flex items-center gap-2">
									{#if tab.icon}
										<span class="relative">
											<tab.icon class="h-4 w-4" />
											{#if tab.notification}
												<span class="absolute -top-1 -right-1 h-2 w-2 rounded-full bg-amber-500"
												></span>
											{/if}
										</span>
									{/if}
									{tab.label}
									{#if tab.count}
										<Tag label={String(tab.count)} color={toColor('amber')} />
									{/if}
								</div>
							</button>
						{/each}
					</nav>
				{/if}
			{/snippet}

			<!-- Content slot -->
			<div class="modal-content">
				{@render children?.(instanceKey)}
			</div>

			{#if footer}
				{@render footer()}
			{:else if keyedActions.length > 0 || footerNote}
				<div class="modal-footer {footerLayout === 'stack' ? 'space-y-4' : ''}">
					{#if keyedActions.length > 0}
						<div
							class="flex flex-wrap items-center gap-3 {startActions.length > 0
								? 'justify-between'
								: 'justify-end'}"
						>
							{#if startActions.length > 0}
								<div class="flex flex-wrap items-center gap-3">
									{#each startActions as keyed, i (i)}
										{@render actionButton(keyed.action, keyed.key)}
									{/each}
								</div>
							{/if}
							<div
								class="flex flex-wrap items-center justify-end gap-3 {footerLayout === 'stack'
									? 'w-full'
									: ''}"
							>
								{#each endActions as keyed, i (i)}
									{@render actionButton(keyed.action, keyed.key)}
								{/each}
							</div>
						</div>
					{/if}
					{@render footerNote?.()}
				</div>
			{/if}
		</div>
	</div>
{/if}

{#snippet actionButton(action: ModalAction, key: string | null)}
	{@const Icon = action.busy ? Loader2 : action.icon}
	{@const paused = key !== null && keyPausedAt(key, focusTarget)}
	<button
		type="button"
		onclick={() => action.onClick()}
		disabled={action.disabled}
		use:tooltip
		data-tooltip={action.tooltip}
		class="{actionClass(action)} {footerLayout === 'stack' && action.role === 'primary'
			? 'flex-1'
			: ''} flex items-center gap-2"
	>
		{#if Icon}
			<Icon class="h-4 w-4 {action.busy ? 'animate-spin' : ''}" />
		{/if}
		{action.label}
		{#if key}
			<!-- Hidden below `md`: a phone has no keyboard to press it with. -->
			<span
				class="hidden transition-opacity md:inline-flex {paused ? 'opacity-40' : ''}"
				title={!paused
					? undefined
					: key === ENTER_KEY
						? common_shortcutWhileTyping({ shortcut: shortcutLabel(keyLabel(ENTER_KEY)) })
						: common_shortcutsPausedWhileTyping()}
			>
				<KbdKey key={keyLabel(key)} size="xs" tone="inherit" />
			</span>
		{/if}
	</button>
{/snippet}

{#snippet entityStep(key: string, targetId: string | null, label: string, side: string)}
	<button
		type="button"
		disabled={!targetId}
		onclick={() => targetId && requestNavigation(targetId)}
		class="fixed {side} top-1/2 z-50 hidden -translate-y-1/2 rounded-sm p-1 text-gray-200 transition-colors hover:text-white disabled:cursor-default disabled:opacity-30 sm:flex"
		aria-label={label}
		title={label}
	>
		<KbdKey key={keyLabel(key)} size="lg" tone="inherit" />
	</button>
{/snippet}

<!-- Only entity modals step between entities, so only they mount the discard dialog. The guard
     also ends the recursion: ConfirmationDialog is itself a GenericModal, with no entity. -->
{#if entityId}
	<ConfirmationDialog
		isOpen={pendingNavigationId !== null}
		title={common_discardChangesTitle()}
		message={common_discardChangesMessage()}
		confirmLabel={common_discard()}
		cancelLabel={common_keepEditing()}
		variant="warning"
		onConfirm={confirmPendingNavigation}
		onCancel={() => (pendingNavigationId = null)}
		onClose={() => (pendingNavigationId = null)}
	/>
{/if}

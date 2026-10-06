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

	/** Open modals, oldest first. Only the last one answers keys shared by every modal. */
	const openModals: symbol[] = [];
</script>

<script lang="ts">
	import { untrack, type Snippet } from 'svelte';
	import { ArrowLeft, X } from 'lucide-svelte';
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
		common_previous
	} from '$lib/paraglide/messages';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';
	import { isEditableTarget, keyLabel } from '$lib/shared/utils/shortcuts';
	import ModalStepper from './ModalStepper.svelte';
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
		onClose = null,
		size = 'lg',
		anchor = 'viewport',
		preventCloseOnClickOutside = false,
		showCloseButton = true,
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
		hasUnsavedChanges = undefined,
		headerIcon,
		banners,
		children,
		footer
	}: {
		title?: string;
		centerTitle?: boolean;
		isOpen?: boolean;
		onClose?: (() => void) | null;
		size?: 'sm' | 'md' | 'lg' | 'xl' | 'full' | 'max';
		/**
		 * What the modal is positioned against. `viewport` covers the whole window; `container`
		 * fills the nearest positioned ancestor instead, for a modal that belongs to one region of
		 * the page rather than to the app.
		 */
		anchor?: 'viewport' | 'container';
		preventCloseOnClickOutside?: boolean;
		showCloseButton?: boolean;
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
		/** Unsaved state the editor keeps outside `form`, such as a host's interface list. */
		hasUnsavedChanges?: () => boolean;
		headerIcon?: Snippet;
		/**
		 * Rendered inside the panel frame, above the title. Opt-in: this component
		 * backs the login, register and share-password modals too, and app banners
		 * have no business over those. Only a modal that gates the app behind it
		 * passes this.
		 */
		banners?: Snippet;
		children?: Snippet<[number]>;
		footer?: Snippet;
	} = $props();

	// With banners in the frame above, the icon and title only compete with them,
	// so the title row stands down. Guarded on `showCloseButton` because the close
	// button lives in that row: a gated modal has none today, and this makes sure
	// no future caller can lose its only way out. The heading itself survives as
	// sr-only below, since `aria-labelledby` points at it.
	let hideTitleRow = $derived(banners != null && !showCloseButton);

	// Tabs and steppers share the title row, between the title and the close button, whenever
	// that row shows a left-aligned title. Otherwise they keep their own row below. In the row,
	// tabs never shrink below their full width; a long title truncates instead.
	let inlineTabs = $derived(tabs.length > 0 && !hideTitleRow && !centerTitle);

	let showBackButton = $derived(
		name != null && $modalState.name === name && $modalState.returnUrl != null
	);
	let returnTitle = $derived(
		name != null && $modalState.name === name ? $modalState.returnTitle : null
	);

	// Track previous open state to detect open transition
	let wasOpen = $state(false);

	// Stacking order: a dialog opened over this one takes Escape and the arrow keys, not both.
	const stackToken = Symbol();
	$effect(() => {
		if (!isOpen) return;
		openModals.push(stackToken);
		return () => {
			openModals.splice(openModals.indexOf(stackToken), 1);
		};
	});

	function isTopmost(): boolean {
		return openModals.at(-1) === stackToken;
	}

	// Left and Right step to the previous and next entity of the on-screen list this modal's
	// entity belongs to. The ends stop rather than wrap.
	let listOrder = $derived(isOpen && name && entityId ? entityListOrderFor(name, entityId) : null);
	let previousId = $derived(entityId ? adjacentEntityId(listOrder, entityId, -1) : null);
	let nextId = $derived(entityId ? adjacentEntityId(listOrder, entityId, 1) : null);

	/** The entity a step is waiting to reach while the discard-changes dialog is up. */
	let pendingNavigationId = $state<string | null>(null);
	/** The entity a step has asked the registry for, until the parent hands it over. */
	let navigatingToId: string | null = null;

	function isDirty(): boolean {
		// Dirty alone stays true after an edit is typed back to the loaded value.
		const formDirty = !!form && form.state.isDirty && !form.state.isDefaultValue;
		return formDirty || (hasUnsavedChanges?.() ?? false);
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
				instanceKey++;
				onOpen?.();
				if (tab && tabs.some((t) => t.id === tab)) activeTab = tab;
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

	// Lock body scroll when modal is open
	$effect(() => {
		if (typeof window !== 'undefined' && isOpen) {
			document.body.style.overflow = 'hidden';
			return () => {
				document.body.style.overflow = '';
			};
		}
	});

	// Sync modal state with URL on open/close transitions
	$effect(() => {
		if (isOpen && !wasOpen) {
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
		full: 'max-w-7xl',
		max: 'max-w-none w-full'
	};

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

	function handleKeydown(event: KeyboardEvent) {
		if (!isOpen || !isTopmost()) return;
		if (event.key === 'Escape') {
			handleClose();
			return;
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

{#if isOpen}
	<!-- Modal backdrop -->
	<div
		class="{showBackdrop ? 'modal-page modal-background' : 'modal-page'} {anchor === 'container'
			? 'modal-page-anchored'
			: ''} {compactPadding ? '!px-2 !py-1 sm:!px-4 sm:!py-4' : ''}"
		onclick={handleBackdropClick}
		role="dialog"
		aria-modal="true"
		aria-labelledby="modal-title"
		onkeydown={(e) => e.key === 'Escape' && isTopmost() && handleClose()}
		tabindex="-1"
	>
		<!-- Floating back button (upper-left of viewport, over backdrop) -->
		{#if showBackButton}
			<button
				type="button"
				onclick={() => goBack()}
				class="fixed left-6 top-6 z-50 flex items-center gap-2 rounded-full bg-white/80 py-2 pl-2 pr-4 text-sm text-gray-600 transition-colors hover:bg-gray-200 hover:text-gray-900 dark:bg-gray-800/80 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
				aria-label={returnTitle ? `Back to ${returnTitle}` : 'Go back'}
			>
				<ArrowLeft class="h-5 w-5" />
				<span class="max-w-48 truncate">{returnTitle ? `Back to ${returnTitle}` : 'Back'}</span>
			</button>
		{/if}

		<!-- Modal content -->
		<div
			class="relative {borderless ? '' : 'modal-container'} {sizeClasses[size]} {compactPadding
				? size === 'full' || fixedHeight
					? 'h-[calc(100vh-1rem)] sm:h-[calc(100vh-2rem)]'
					: 'max-h-[calc(100vh-1rem)] sm:max-h-[calc(100vh-2rem)]'
				: size === 'full' || fixedHeight
					? 'h-[calc(100vh-2rem)] sm:h-[calc(100vh-8rem)]'
					: 'max-h-[calc(100vh-2rem)] sm:max-h-[calc(100vh-8rem)]'} flex w-full flex-col"
		>
			<!-- Floating close button (absolute positioned within modal container) -->
			{#if floatingCloseButton && onClose}
				<button
					type="button"
					onclick={handleClose}
					class="absolute right-4 top-3 z-30 rounded-full bg-white/80 p-2 text-gray-600 transition-colors hover:bg-gray-200 hover:text-gray-900 dark:bg-gray-800/80 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
					aria-label={common_closeModal()}
				>
					<X class="h-5 w-5" />
				</button>
			{/if}
			{#if banners}
				<!-- Clipped here rather than by putting overflow-hidden on the panel,
				     which ~42 modals share: AppBanner is full-bleed with square corners
				     and the panel is rounded. -->
				<div class="shrink-0 overflow-hidden rounded-t-lg">
					{@render banners()}
				</div>
			{/if}
			{#if hideTitleRow}
				<!-- Keeps the dialog's accessible name while the row is stood down. -->
				<h2 id="modal-title" class="sr-only">{title}</h2>
			{/if}
			<!-- Header (hidden when no title, no close button, and no tabs) -->
			{#if title || showCloseButton || tabs.length > 0}
				<div class="modal-header flex-col gap-0 {tabs.length > 0 && !inlineTabs ? 'pb-0' : ''}">
					<!-- Title row -->
					{#if !hideTitleRow}
						<div class="flex w-full items-center justify-between {inlineTabs ? 'gap-6' : ''}">
							{#if centerTitle}
								{@render headerIcon?.()}
								<h2
									id="modal-title"
									class="text-primary absolute left-1/2 max-w-[calc(100%-5rem)] -translate-x-1/2 text-center text-xl font-semibold"
								>
									{title}
								</h2>
							{:else}
								<div class="flex min-w-0 items-center gap-3 {inlineTabs ? 'mr-4' : ''}">
									{@render headerIcon?.()}
									<h2 id="modal-title" class="text-primary truncate text-xl font-semibold">
										{title}
									</h2>
								</div>
							{/if}

							{#if inlineTabs}
								{@render tabNav(true)}
							{/if}

							{#if listOrder}
								<div class="ml-auto mr-2 flex shrink-0 items-center gap-1">
									<button
										type="button"
										class="btn-icon p-1 disabled:opacity-40"
										disabled={!previousId}
										onclick={() => previousId && requestNavigation(previousId)}
										aria-label={common_previous()}
										title={common_previous()}
									>
										<KbdKey key={keyLabel('ArrowLeft')} size="sm" />
									</button>
									<button
										type="button"
										class="btn-icon p-1 disabled:opacity-40"
										disabled={!nextId}
										onclick={() => nextId && requestNavigation(nextId)}
										aria-label={common_next()}
										title={common_next()}
									>
										<KbdKey key={keyLabel('ArrowRight')} size="sm" />
									</button>
								</div>
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
					<nav
						class={inline ? 'flex flex-1 items-center gap-6' : 'flex w-full space-x-6 pt-4'}
						aria-label={common_modalTabs()}
					>
						{#each tabs as tab (tab.id)}
							<button
								type="button"
								onclick={() => !tab.disabled && handleTabClick(tab.id)}
								class="shrink-0 whitespace-nowrap border-b-2 px-1 text-sm font-medium transition-colors
									{inline ? 'py-1' : 'pb-3'}
									{tab.disabled
									? 'text-muted cursor-not-allowed border-transparent opacity-50'
									: activeTab === tab.id
										? 'text-primary border-blue-500'
										: 'text-muted hover:text-secondary border-transparent'}"
								aria-current={activeTab === tab.id ? 'page' : undefined}
								aria-disabled={tab.disabled ? 'true' : undefined}
							>
								<div class="flex items-center gap-2">
									{#if tab.icon}
										<span class="relative">
											<tab.icon class="h-4 w-4" />
											{#if tab.notification}
												<span class="absolute -right-1 -top-1 h-2 w-2 rounded-full bg-amber-500"
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

			<!-- Footer slot -->
			{@render footer?.()}
		</div>
	</div>
{/if}

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

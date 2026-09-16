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
		disabled?: boolean;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ArrowLeft, X } from 'lucide-svelte';
	import { common_closeModal, common_modal, common_modalTabs } from '$lib/paraglide/messages';
	import ModalStepper from './ModalStepper.svelte';
	import { get } from 'svelte/store';
	import {
		modalState,
		openModal,
		closeModal,
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
		headerIcon,
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
		headerIcon?: Snippet;
		children?: Snippet<[number]>;
		footer?: Snippet;
	} = $props();

	let showBackButton = $derived(
		name != null && $modalState.name === name && $modalState.returnUrl != null
	);
	let returnTitle = $derived(
		name != null && $modalState.name === name ? $modalState.returnTitle : null
	);

	// Track previous open state to detect open transition
	let wasOpen = $state(false);

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
				// Read subEntityId, returnUrl, returnTitle before openModal clears them
				const subEntityId = state.name === name ? state.subEntityId : null;
				const returnUrl = state.name === name ? (state.returnUrl ?? undefined) : undefined;
				const savedReturnTitle = state.name === name ? (state.returnTitle ?? undefined) : undefined;
				openModal(name, {
					id: entityId,
					tab: activeTab || undefined,
					returnUrl,
					returnTitle: savedReturnTitle
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
		if (event.key === 'Escape' && isOpen) {
			handleClose();
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
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
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
			<!-- Header (hidden when no title, no close button, and no tabs) -->
			{#if title || showCloseButton || tabs.length > 0}
				<div class="modal-header flex-col gap-0 {tabs.length > 0 ? 'pb-0' : ''}">
					<!-- Title row -->
					<div class="flex w-full items-center justify-between">
						{#if centerTitle}
							{@render headerIcon?.()}
							<h2
								id="modal-title"
								class="text-primary absolute left-1/2 max-w-[calc(100%-5rem)] -translate-x-1/2 text-center text-xl font-semibold"
							>
								{title}
							</h2>
						{:else}
							<div class="flex items-center gap-3">
								{@render headerIcon?.()}
								<h2 id="modal-title" class="text-primary text-xl font-semibold">
									{title}
								</h2>
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

					<!-- Tab navigation (if tabs provided) -->
					{#if tabs.length > 0 && tabStyle === 'stepper'}
						<ModalStepper {tabs} {activeTab} onTabClick={handleTabClick} />
					{:else if tabs.length > 0}
						<nav class="flex w-full space-x-6 pt-4" aria-label={common_modalTabs()}>
							{#each tabs as tab (tab.id)}
								<button
									type="button"
									onclick={() => !tab.disabled && handleTabClick(tab.id)}
									class="border-b-2 px-1 pb-3 text-sm font-medium transition-colors
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
									</div>
								</button>
							{/each}
						</nav>
					{/if}
				</div>
			{/if}

			<!-- Content slot -->
			<div class="modal-content">
				{@render children?.(instanceKey)}
			</div>

			<!-- Footer slot -->
			{@render footer?.()}
		</div>
	</div>
{/if}

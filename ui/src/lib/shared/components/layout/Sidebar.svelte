<script lang="ts">
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import SettingsModal from '$lib/features/settings/SettingsModal.svelte';
	import SupportModal from '$lib/features/support/SupportModal.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { modalState, openModal } from '$lib/shared/stores/modal-registry';
	import type { IconComponent } from '$lib/shared/utils/types';
	import {
		Menu,
		ChevronDown,
		History,
		Calendar,
		Settings,
		LifeBuoy,
		ArrowUpCircle,
		Home
	} from 'lucide-svelte';
	import { onMount } from 'svelte';
	import type { Component } from 'svelte';
	import type { UserOrgPermissions } from '$lib/features/users/types';
	import { trackEvent } from '$lib/shared/utils/analytics';

	// Import tab components
	import TopologyTab from '$lib/features/topology/components/TopologyTab.svelte';
	import DiscoverySessionTab from '$lib/features/discovery/components/tabs/DiscoverySessionTab.svelte';
	import DiscoveryScheduledTab from '$lib/features/discovery/components/tabs/DiscoveryScheduledTab.svelte';
	import DiscoveryHistoryTab from '$lib/features/discovery/components/tabs/DiscoveryHistoryTab.svelte';
	import NetworksTab from '$lib/features/networks/components/NetworksTab.svelte';
	import SubnetTab from '$lib/features/subnets/components/SubnetTab.svelte';
	import GroupTab from '$lib/features/groups/components/GroupTab.svelte';
	import HostTab from '$lib/features/hosts/components/HostTab.svelte';
	import ServiceTab from '$lib/features/services/components/ServiceTab.svelte';
	import DaemonTab from '$lib/features/daemons/components/DaemonTab.svelte';
	import ApiKeyTab from '$lib/features/daemon_api_keys/components/ApiKeyTab.svelte';
	import UserTab from '$lib/features/users/components/UserTab.svelte';
	import UserApiKeyTab from '$lib/features/user_api_keys/components/UserApiKeyTab.svelte';
	import TagTab from '$lib/features/tags/components/TagTab.svelte';
	import SnmpCredentialsTab from '$lib/features/snmp/components/SnmpCredentialsTab.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import ShareTab from '$lib/features/shares/components/ShareTab.svelte';
	import HomeTab from '$lib/features/home/components/HomeTab.svelte';

	let {
		activeTab = $bindable('topology'),
		collapsed = $bindable(false),
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		allTabs = $bindable<Array<{ id: string; component: any; isReadOnly: boolean }>>([]),
		showSettings = $bindable(false),
		settingsInitialTab = 'account',
		settingsDismissible = true
	}: {
		activeTab?: string;
		collapsed?: boolean;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		allTabs?: Array<{ id: string; component: any; isReadOnly: boolean }>;
		showSettings?: boolean;
		settingsInitialTab?: string;
		settingsDismissible?: boolean;
	} = $props();

	// TanStack Query for current user and organization
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Derived values from queries
	let userPermissions = $derived(currentUser?.permissions);
	let isBillingEnabled = $derived(organization ? isBillingPlanActive(organization) : false);
	let isDemoOrg = $derived(organization?.plan?.type === 'Demo');
	let isFreePlan = $derived(organization?.plan?.type === 'Free');
	let isOwner = $derived(userPermissions === 'Owner');
	let showUpgradeButton = $derived(isFreePlan && isOwner && isBillingEnabled);
	let isReadOnly = $derived(userPermissions === 'Viewer');

	let showSupport = $state(false);

	// Show notification on settings only when trialing without payment method
	// Free plan users don't need a payment method, so no dot for them
	let showBillingNotification = $derived.by(() => {
		if (!organization) return false;
		const isPastDue = organization.plan_status === 'past_due';
		const isTrialing = organization.plan_status === 'trialing';
		const hasPayment = organization.has_payment_method ?? false;
		return isPastDue || (isTrialing && !hasPayment);
	});

	// Sync settings/support modal state from modal registry (for deep-link opens)
	$effect(() => {
		if ($modalState.name === 'settings' && !showSettings) {
			settingsInitialTab = $modalState.tab ?? 'account';
			showSettings = true;
		}
		if ($modalState.name === 'support' && !showSupport) {
			showSupport = true;
		}
	});

	interface NavItem {
		id: string;
		label: string;
		icon: IconComponent;
		component?: Component;
		position?: 'main' | 'bottom';
		onClick?: () => void | Promise<void>;
		requiredPermissions?: UserOrgPermissions[]; // Which permissions can see this item. If empty, Viewer+ is allowed.
		requiresBilling?: boolean; // Whether this requires billing to be enabled
		hideInDemo?: boolean; // Whether to hide this in demo mode
		children?: NavItem[]; // Nested child items (displayed indented under parent)
	}

	interface NavSection {
		id: string;
		label: string;
		items: NavItem[];
		position?: 'main' | 'bottom';
	}

	type NavConfig = (NavSection | NavItem)[];

	const SIDEBAR_STORAGE_KEY = 'scanopy-sidebar-collapsed';

	// Base navigation config (before filtering)
	const baseNavConfig: NavConfig = [
		{
			id: 'home',
			label: 'Home',
			icon: Home as IconComponent,
			component: HomeTab
		},
		{
			id: 'visualize',
			label: 'Visualize',
			items: [
				{
					id: 'topology',
					label: 'Topology',
					icon: entities.getIconComponent('Topology'),
					component: TopologyTab
				},
				{
					id: 'groups',
					label: 'Groups',
					icon: entities.getIconComponent('Group'),
					component: GroupTab
				},
				{
					id: 'shares',
					label: 'Sharing',
					icon: entities.getIconComponent('Share'),
					component: ShareTab
				}
			]
		},
		{
			id: 'discover',
			label: 'Discover',
			items: [
				{
					id: 'discovery-sessions',
					label: 'Sessions',
					icon: entities.getIconComponent('Discovery'),
					component: DiscoverySessionTab
				},
				{
					id: 'discovery-scheduled',
					label: 'Scheduled',
					icon: Calendar as IconComponent,
					component: DiscoveryScheduledTab
				},
				{
					id: 'discovery-history',
					label: 'History',
					icon: History as IconComponent,
					component: DiscoveryHistoryTab
				},
				{
					id: 'daemons',
					label: 'Daemons',
					icon: entities.getIconComponent('Daemon'),
					component: DaemonTab,
					children: [
						{
							id: 'daemon-api-keys',
							label: 'Api Keys',
							icon: entities.getIconComponent('DaemonApiKey'),
							component: ApiKeyTab,
							requiredPermissions: ['Member', 'Admin', 'Owner']
						}
					]
				}
			]
		},
		{
			id: 'assets',
			label: 'Assets',
			items: [
				{
					id: 'networks',
					label: 'Networks',
					icon: entities.getIconComponent('Network'),
					component: NetworksTab
				},
				{
					id: 'subnets',
					label: 'Subnets',
					icon: entities.getIconComponent('Subnet'),
					component: SubnetTab
				},
				{
					id: 'hosts',
					label: 'Hosts',
					icon: entities.getIconComponent('Host'),
					component: HostTab
				},
				{
					id: 'services',
					label: 'Services',
					icon: entities.getIconComponent('Service'),
					component: ServiceTab
				}
			]
		},
		{
			id: 'platform',
			label: 'Platform',
			items: [
				{
					id: 'tags',
					label: 'Tags',
					icon: entities.getIconComponent('Tag'),
					component: TagTab
				},
				{
					id: 'users',
					label: 'Users',
					icon: entities.getIconComponent('User'),
					component: UserTab,
					requiredPermissions: ['Admin', 'Owner']
				},
				{
					id: 'api-keys',
					label: 'API Keys',
					icon: entities.getIconComponent('UserApiKey'),
					component: UserApiKeyTab,
					requiredPermissions: ['Member', 'Admin', 'Owner']
				},
				{
					id: 'snmp-credentials',
					label: 'SNMP Credentials',
					icon: entities.getIconComponent('SnmpCredential'),
					component: SnmpCredentialsTab
				}
			]
		},
		{
			id: 'settings',
			label: 'Settings',
			icon: Settings as IconComponent,
			position: 'bottom',
			onClick: async () => {
				showSettings = true;
			}
		},
		{
			id: 'support',
			label: 'Support',
			icon: LifeBuoy,
			position: 'bottom',
			onClick: async () => {
				showSupport = true;
			}
		}
	];

	// Extract all tabs with components from the filtered nav config and expose to parent
	// Use navConfig (filtered by permissions) instead of baseNavConfig to prevent
	// instantiating components the user doesn't have permission to access
	$effect(() => {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const tabs: Array<{ id: string; component: any; isReadOnly: boolean }> = [];

		// Helper to extract tabs from an item and its children
		function extractTabsFromItem(item: NavItem) {
			if (item.component) {
				tabs.push({ id: item.id, component: item.component, isReadOnly });
			}
			// Also extract tabs from children
			if (item.children) {
				for (const child of item.children) {
					extractTabsFromItem(child);
				}
			}
		}

		for (const configItem of navConfig) {
			if (isSection(configItem)) {
				// Get tabs from section items
				for (const item of configItem.items) {
					extractTabsFromItem(item);
				}
			} else {
				// Standalone item
				extractTabsFromItem(configItem);
			}
		}

		allTabs = tabs;
	});

	// Helper to check if user has required permissions
	function hasRequiredPermissions(item: NavItem): boolean {
		// If no permissions specified, everyone can see it
		if (!item.requiredPermissions || item.requiredPermissions.length === 0) {
			return true;
		}

		// If user has no permissions, they can't see items with permission requirements
		if (!userPermissions) {
			return false;
		}

		// Check if user's permission is in the allowed list
		return item.requiredPermissions.includes(userPermissions);
	}

	// Helper to check billing requirements
	function meetsBillingRequirement(item: NavItem): boolean {
		// If billing not required, always show
		if (!item.requiresBilling) {
			return true;
		}

		// If billing is required, check if it's enabled
		return isBillingEnabled;
	}

	// Helper to check demo mode requirements
	function meetsDemoModeRequirements(item: NavItem): boolean {
		// Only hide items marked hideInDemo when in demo mode and user is not Owner
		if (item.hideInDemo && isDemoOrg && userPermissions != 'Owner') {
			return false;
		}
		return true;
	}

	// Helper to check if item should be visible
	function isItemVisible(item: NavItem): boolean {
		return (
			hasRequiredPermissions(item) &&
			meetsBillingRequirement(item) &&
			meetsDemoModeRequirements(item)
		);
	}

	// Helper to filter an item and its children
	function filterItemWithChildren(item: NavItem): NavItem | null {
		if (!isItemVisible(item)) {
			return null;
		}

		// If item has children, filter them too
		if (item.children) {
			const visibleChildren = item.children.filter(isItemVisible);
			return {
				...item,
				children: visibleChildren.length > 0 ? visibleChildren : undefined
			};
		}

		return item;
	}

	// Filter nav config based on user permissions and billing status
	let navConfig = $derived.by((): NavConfig => {
		return baseNavConfig
			.map((configItem) => {
				if (isSection(configItem)) {
					// Filter items within the section (including their children)
					const visibleItems = configItem.items
						.map(filterItemWithChildren)
						.filter((item): item is NavItem => item !== null);

					// Only include section if it has visible items
					if (visibleItems.length === 0) {
						return null;
					}

					return {
						...configItem,
						items: visibleItems
					};
				} else {
					// Standalone item - check if it should be visible
					return filterItemWithChildren(configItem);
				}
			})
			.filter((item): item is NavSection | NavItem => item !== null);
	});

	// Track collapsed state for each section
	let sectionStates = $state<Record<string, boolean>>({});

	// Helper to check if item is a section
	function isSection(item: NavSection | NavItem): item is NavSection {
		return 'items' in item;
	}

	// Filter nav items by position
	function filterByPosition(items: NavConfig, position: 'main' | 'bottom'): NavConfig {
		return items.filter((item) => {
			const itemPosition = isSection(item) ? item.position : item.position;
			return itemPosition === position || (position === 'main' && !itemPosition);
		});
	}

	let mainNavItems = $derived(filterByPosition(navConfig, 'main'));
	let bottomNavItems = $derived(filterByPosition(navConfig, 'bottom'));

	onMount(() => {
		if (typeof window !== 'undefined') {
			try {
				const stored = localStorage.getItem(SIDEBAR_STORAGE_KEY);
				if (stored !== null) {
					collapsed = JSON.parse(stored);
				}

				// Load section states
				baseNavConfig.forEach((item) => {
					if (isSection(item)) {
						const key = `scanopy-section-${item.id}-collapsed`;
						const sectionStored = localStorage.getItem(key);
						if (sectionStored !== null) {
							sectionStates[item.id] = JSON.parse(sectionStored);
						} else {
							sectionStates[item.id] = false; // Default expanded
						}
					}
				});
			} catch (error) {
				console.warn('Failed to load sidebar state from localStorage:', error);
			}
		}
	});

	function toggleCollapse() {
		collapsed = !collapsed;

		// Save to localStorage
		if (typeof window !== 'undefined') {
			try {
				localStorage.setItem(SIDEBAR_STORAGE_KEY, JSON.stringify(collapsed));
			} catch (error) {
				console.error('Failed to save sidebar state to localStorage:', error);
			}
		}
	}

	function toggleSection(sectionId: string) {
		sectionStates[sectionId] = !sectionStates[sectionId];

		if (typeof window !== 'undefined') {
			try {
				const key = `scanopy-section-${sectionId}-collapsed`;
				localStorage.setItem(key, JSON.stringify(sectionStates[sectionId]));
			} catch (error) {
				console.error('Failed to save section state:', error);
			}
		}
	}

	function handleItemClick(item: NavItem) {
		if (item.onClick) {
			item.onClick();
		} else {
			activeTab = item.id;
		}
	}

	const inactiveButtonClass =
		'text-tertiary hover:text-secondary hover:bg-gray-800 border border-[#15131e]';

	const sectionHeaderClass =
		'text-secondary hover:text-primary flex w-full items-center rounded-lg text-xs font-semibold uppercase tracking-wide transition-colors hover:bg-gray-800/50';

	const baseClasses = 'flex w-full items-center rounded-lg font-medium transition-colors';
</script>

<div
	class="sidebar flex flex-shrink-0 flex-col transition-all duration-300"
	class:w-16={collapsed}
	class:w-64={!collapsed}
>
	<!-- Logo/Brand -->
	<div class="flex min-h-0 flex-1 flex-col">
		<div class="border-b border-gray-700 px-2 py-4">
			<button
				onclick={toggleCollapse}
				class="text-tertiary hover:text-secondary flex w-full items-center rounded-lg transition-colors hover:bg-gray-800"
				style="height: 2.5rem; padding: 0.5rem 0.75rem;"
				aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
			>
				<Menu class="h-5 w-5 flex-shrink-0" />
				{#if !collapsed}
					<div class="absolute left-1/2 flex -translate-x-1/2 transform items-center">
						<img src="/logos/scanopy-logo.png" alt="Logo" class="h-8 w-auto" />
						<h1 class="text-primary ml-3 truncate whitespace-nowrap text-xl font-bold">Scanopy</h1>
					</div>
				{/if}
			</button>
			{#if !collapsed && isDemoOrg}
				<div class="mt-2 flex justify-center">
					<Tag label="Demo" color="Yellow" />
				</div>
			{/if}
		</div>

		<!-- Main Navigation -->
		<nav class="flex-1 overflow-y-auto px-2 py-4">
			<ul class="space-y-4">
				{#each mainNavItems as configItem (configItem.id)}
					{#if isSection(configItem)}
						<!-- Section with items -->
						<li>
							{#if !collapsed}
								<button
									onclick={() => toggleSection(configItem.id)}
									class={sectionHeaderClass}
									style="height: 2rem; padding: 0.25rem 0.75rem;"
								>
									<span class="flex-1 text-left">{configItem.label}</span>
									<ChevronDown
										class="h-4 w-4 flex-shrink-0 transition-transform {sectionStates[configItem.id]
											? '-rotate-90'
											: ''}"
									/>
								</button>
							{/if}

							{#if !sectionStates[configItem.id] || collapsed}
								<ul class="mt-1 space-y-1" class:mt-0={collapsed}>
									{#each configItem.items as item (item.id)}
										<li>
											<button
												onclick={() => handleItemClick(item)}
												class="{baseClasses} {activeTab === item.id
													? 'text-primary border border-blue-600 bg-blue-700'
													: inactiveButtonClass}"
												style="height: 2.5rem; padding: 0.5rem 0.75rem;"
												title={collapsed ? item.label : ''}
											>
												<item.icon class="h-5 w-5 flex-shrink-0" />
												{#if !collapsed}
													<span class="ml-3 truncate">{item.label}</span>
												{/if}
											</button>
											<!-- Render children if present -->
											{#if item.children && item.children.length > 0}
												<ul class="mt-1 space-y-1" class:ml-4={!collapsed}>
													{#each item.children as child (child.id)}
														<li>
															<button
																onclick={() => handleItemClick(child)}
																class="{baseClasses} {activeTab === child.id
																	? 'text-primary border border-blue-600 bg-blue-700'
																	: inactiveButtonClass}"
																style="height: 2.25rem; padding: 0.375rem 0.75rem;"
																title={collapsed ? child.label : ''}
															>
																<child.icon class="h-4 w-4 flex-shrink-0" />
																{#if !collapsed}
																	<span class="ml-3 truncate text-sm">{child.label}</span>
																{/if}
															</button>
														</li>
													{/each}
												</ul>
											{/if}
										</li>
									{/each}
								</ul>
							{/if}
						</li>
					{:else}
						<!-- Standalone item (no section, no indentation) -->
						<li>
							<button
								onclick={() => handleItemClick(configItem)}
								class="{baseClasses} {activeTab === configItem.id ||
								(configItem.id === 'settings' && showSettings)
									? 'text-primary border border-blue-600 bg-blue-700'
									: inactiveButtonClass}"
								style="height: 2.5rem; padding: 0.5rem 0.75rem;"
								title={collapsed ? configItem.label : ''}
							>
								<configItem.icon class="h-5 w-5 flex-shrink-0" />
								{#if !collapsed}
									<span class="ml-3 truncate">{configItem.label}</span>
								{/if}
							</button>
						</li>
					{/if}
				{/each}
			</ul>
		</nav>
	</div>

	<!-- Bottom Navigation -->
	<div class="flex-shrink-0 border-t border-gray-700 px-2 py-2">
		<ul class="space-y-1">
			{#if showUpgradeButton}
				<li>
					<button
						class="{baseClasses} text-amber-400 hover:bg-amber-500/10"
						style="height: 2.5rem; padding: 0.5rem 0.75rem;"
						title={collapsed ? 'Upgrade' : ''}
						onclick={() => {
							trackEvent('upgrade_button_clicked', { feature: 'sidebar' });
							openModal('billing-plan');
						}}
					>
						<ArrowUpCircle class="h-5 w-5 flex-shrink-0" />
						{#if !collapsed}
							<span class="ml-3 truncate">Upgrade</span>
						{/if}
					</button>
				</li>
			{/if}
			{#each bottomNavItems as item (item.id)}
				{#if !isSection(item)}
					<li>
						<button
							onclick={() => handleItemClick(item)}
							class="{baseClasses} {activeTab === item.id ||
							(item.id === 'settings' && showSettings)
								? 'text-primary border border-blue-600 bg-blue-700'
								: inactiveButtonClass}"
							style="height: 2.5rem; padding: 0.5rem 0.75rem;"
							title={collapsed ? item.label : ''}
						>
							<span class="relative">
								<item.icon class="h-5 w-5 flex-shrink-0" />
								{#if item.id === 'settings' && showBillingNotification}
									<span class="absolute -right-1 -top-1 h-2.5 w-2.5 rounded-full bg-amber-500"
									></span>
								{/if}
							</span>
							{#if !collapsed}
								<span class="ml-3 truncate">{item.label}</span>
							{/if}
						</button>
					</li>
				{/if}
			{/each}
		</ul>
	</div>
</div>

<SettingsModal
	isOpen={showSettings}
	name="settings"
	onClose={() => (showSettings = false)}
	initialTab={settingsInitialTab}
	dismissible={settingsDismissible}
/>
<SupportModal isOpen={showSupport} name="support" onClose={() => (showSupport = false)} />

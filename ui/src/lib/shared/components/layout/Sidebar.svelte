<script lang="ts">
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import SettingsModal from '$lib/features/settings/SettingsModal.svelte';
	import { isEmbed } from '$lib/shared/utils/embed';
	import { getLocale } from '$lib/paraglide/runtime';

	// 侧栏菜单文案未走 paraglide(TAB_LABELS 静态 + 分组硬编码英文),这里补中文映射。
	const MENU_ZH: Record<string, string> = {
		Home: '首页',
		Topology: '拓扑',
		Sharing: '共享',
		Scans: '扫描',
		Historical: '历史',
		Daemons: '采集器',
		'API Keys': 'API 密钥',
		Networks: '网络',
		Subnets: '子网',
		Hosts: '主机',
		Services: '服务',
		Tags: '标签',
		Users: '用户',
		Credentials: '凭据',
		Discover: '发现',
		Assets: '资产',
		Platform: '平台',
		Settings: '设置',
		Support: '支持'
	};
	function mLabel(en: string): string {
		return getLocale() === 'zh' ? (MENU_ZH[en] ?? en) : en;
	}
	import SupportModal from '$lib/features/support/SupportModal.svelte';
	import { billingPlans, entities } from '$lib/shared/stores/metadata';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import { modalState } from '$lib/shared/stores/modal-registry';
	import { entityUIConfig, TAB_LABELS } from '$lib/shared/entity-ui-config';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import type { IconComponent } from '$lib/shared/utils/types';
	import {
		Menu,
		ChevronDown,
		History,
		Settings,
		LifeBuoy,
		ArrowUpCircle,
		Clock,
		Home
	} from 'lucide-svelte';
	import { onMount } from 'svelte';
	import type { Component } from 'svelte';
	import type { UserOrgPermissions } from '$lib/features/users/types';
	import type { SubTab } from '$lib/shared/components/layout/ContentSubTabs.svelte';
	import {
		common_upgrade,
		billing_trialPill,
		billing_trialPillOneDay,
		billing_trialPillToday
	} from '$lib/paraglide/messages';
	import {
		getTrialDaysLeft,
		isTrialingWithoutPayment,
		isMissingPaymentMethod
	} from '$lib/shared/utils/trial';
	import { daemonSetupState } from '$lib/features/daemons/stores/daemon-setup';
	import { isAllComplete } from '$lib/shared/onboarding/checklist';
	import SidebarChecklist from './SidebarChecklist.svelte';
	import type { components } from '$lib/api/schema';

	// Import tab components
	import TopologyTab from '$lib/features/topology/components/TopologyTab.svelte';
	import DiscoveryScheduledTab from '$lib/features/discovery/components/tabs/DiscoveryScheduledTab.svelte';
	import DiscoveryHistoryTab from '$lib/features/discovery/components/tabs/DiscoveryHistoryTab.svelte';
	import NetworksTab from '$lib/features/networks/components/NetworksTab.svelte';
	import SubnetTab from '$lib/features/subnets/components/SubnetTab.svelte';
	import HostTab from '$lib/features/hosts/components/HostTab.svelte';
	import ServiceTab from '$lib/features/services/components/ServiceTab.svelte';
	import DaemonTab from '$lib/features/daemons/components/DaemonTab.svelte';
	import ApiKeyTab from '$lib/features/daemon_api_keys/components/ApiKeyTab.svelte';
	import UserTab from '$lib/features/users/components/UserTab.svelte';
	import UserApiKeyTab from '$lib/features/user_api_keys/components/UserApiKeyTab.svelte';
	import TagTab from '$lib/features/tags/components/TagTab.svelte';
	import CredentialsTab from '$lib/features/credentials/components/CredentialsTab.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';

	import HomeTab from '$lib/features/home/components/HomeTab.svelte';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	let {
		activeTab = $bindable('topology'),
		collapsed = $bindable(false),
		// eslint-disable-next-line no-useless-assignment
		allTabs = $bindable<
			Array<{
				id: string;
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				component: any;
				isReadOnly: boolean;
				subTabIds?: string[];
				subTabDefs?: SubTab[];
				subTabNotifications?: Record<string, string>;
			}>
		>([]),
		showSettings = $bindable(false),
		settingsInitialTab = 'account',
		settingsDismissible = true
	}: {
		activeTab?: string;
		collapsed?: boolean;
		allTabs?: Array<{
			id: string;
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			component: any;
			isReadOnly: boolean;
			subTabIds?: string[];
			subTabDefs?: SubTab[];
			subTabNotifications?: Record<string, string>;
		}>;
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
	let planMeta = $derived(billingPlans.getMetadata(organization?.plan?.type ?? null));
	let isDemoOrg = $derived(planMeta.is_demo === true);
	let isFreePlan = $derived(planMeta.is_free === true);
	let isOwner = $derived(userPermissions === 'Owner');
	let trialDaysLeft = $derived(getTrialDaysLeft(organization));
	let showTrialPill = $derived(
		isOwner &&
			isBillingEnabled &&
			isTrialingWithoutPayment(organization) &&
			trialDaysLeft !== null &&
			trialDaysLeft <= 7
	);
	let showFreeUpgradeButton = $derived(!isEmbed && isFreePlan && isOwner && isBillingEnabled);
	let trialPillLabel = $derived.by(() => {
		if (trialDaysLeft === null) return '';
		if (trialDaysLeft <= 0) return billing_trialPillToday();
		if (trialDaysLeft === 1) return billing_trialPillOneDay();
		return billing_trialPill({ days: trialDaysLeft });
	});
	let isReadOnly = $derived(userPermissions === 'Viewer');

	let showSupport = $state(false);

	// Show notification on settings only when trialing without payment method
	// Free plan users don't need a payment method, so no dot for them.
	// Paused is intentionally excluded — the hard-gate billing modal already
	// blocks the entire app, so a sidebar dot would be noise.
	let showBillingNotification = $derived.by(() => {
		if (!organization) return false;
		// past_due stays its own clause: dunning needs attention even with a card
		// on file. isMissingPaymentMethod is the shared no-card predicate, so this
		// dot, the banner, and the BillingTab card all key off the same rule.
		return organization.plan_status === 'past_due' || isMissingPaymentMethod(organization);
	});

	// Active discovery sessions — used for notification dot on sidebar and sub-tabs
	const activeSessionsQuery = useActiveSessionsQuery(() => true);
	let hasActiveSessions = $derived((activeSessionsQuery.data?.length ?? 0) > 0);

	// Onboarding checklist state
	let onboarding = $derived((organization?.onboarding ?? []) as OnboardingOperation[]);
	let showSidebarChecklist = $derived(organization && !isAllComplete(onboarding));
	let isDiscoveryActive = $derived(
		(activeSessionsQuery.data ?? []).some(
			(s) => s.discovery_type?.type === 'Network' || s.discovery_type?.type === 'Unified'
		)
	);

	// Subscribe to daemon setup state
	let daemonStatus = $state<'idle' | 'waiting' | 'connected' | 'trouble'>('idle');
	const unsubscribeDaemon = daemonSetupState.subscribe((s) => {
		daemonStatus = s.connectionStatus;
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
		entityType?: EntityDiscriminants; // Links this nav item to an entity type via entityUIConfig
		component?: Component;
		position?: 'main' | 'bottom';
		onClick?: () => void | Promise<void>;
		requiredPermissions?: UserOrgPermissions[]; // Which permissions can see this item. If empty, Viewer+ is allowed.
		requiresBilling?: boolean; // Whether this requires billing to be enabled
		hideInDemo?: boolean; // Whether to hide this in demo mode
		children?: NavItem[]; // Nested child items (displayed indented under parent)
		subTabs?: (SubTab & { requiredPermissions?: UserOrgPermissions[] })[]; // Content area sub-tabs
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
			label: TAB_LABELS['home'],
			icon: Home as IconComponent,
			component: HomeTab
		},
		{
			id: entityUIConfig.Topology!.tabId,
			label: TAB_LABELS[entityUIConfig.Topology!.tabId],
			icon: entities.getIconComponent('Topology'),
			entityType: 'Topology',
			component: TopologyTab
		},
		{
			id: 'discover',
			label: 'Discover',
			items: [
				{
					id: 'discovery',
					label: TAB_LABELS['discovery'],
					icon: entities.getIconComponent('Discovery'),
					subTabs: [
						{
							id: entityUIConfig.Discovery!.tabId,
							label: TAB_LABELS[entityUIConfig.Discovery!.tabId],
							icon: entities.getIconComponent('Discovery'),
							component: DiscoveryScheduledTab
						},
						{
							id: 'discovery-history',
							label: TAB_LABELS['discovery-history'],
							icon: History as IconComponent,
							component: DiscoveryHistoryTab
						}
					]
				},
				{
					id: 'daemons-group',
					label: TAB_LABELS[entityUIConfig.Daemon!.tabId],
					icon: entities.getIconComponent('Daemon'),
					subTabs: [
						{
							id: entityUIConfig.Daemon!.tabId,
							label: TAB_LABELS[entityUIConfig.Daemon!.tabId],
							icon: entities.getIconComponent('Daemon'),
							component: DaemonTab
						},
						{
							id: entityUIConfig.DaemonApiKey!.tabId,
							label: TAB_LABELS[entityUIConfig.DaemonApiKey!.tabId],
							icon: entities.getIconComponent('DaemonApiKey'),
							component: ApiKeyTab,
							requiredPermissions: ['Member', 'Admin', 'Owner'] as UserOrgPermissions[]
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
					id: entityUIConfig.Network!.tabId,
					label: TAB_LABELS[entityUIConfig.Network!.tabId],
					icon: entities.getIconComponent('Network'),
					entityType: 'Network',
					component: NetworksTab
				},
				{
					id: entityUIConfig.Subnet!.tabId,
					label: TAB_LABELS[entityUIConfig.Subnet!.tabId],
					icon: entities.getIconComponent('Subnet'),
					entityType: 'Subnet',
					component: SubnetTab
				},
				{
					id: entityUIConfig.Host!.tabId,
					label: TAB_LABELS[entityUIConfig.Host!.tabId],
					icon: entities.getIconComponent('Host'),
					entityType: 'Host',
					component: HostTab
				},
				{
					id: entityUIConfig.Service!.tabId,
					label: TAB_LABELS[entityUIConfig.Service!.tabId],
					icon: entities.getIconComponent('Service'),
					entityType: 'Service',
					component: ServiceTab
				}
			]
		},
		{
			id: 'platform',
			label: 'Platform',
			items: [
				{
					id: entityUIConfig.Tag!.tabId,
					label: TAB_LABELS[entityUIConfig.Tag!.tabId],
					icon: entities.getIconComponent('Tag'),
					entityType: 'Tag',
					component: TagTab
				},
				{
					id: entityUIConfig.User!.tabId,
					label: TAB_LABELS[entityUIConfig.User!.tabId],
					icon: entities.getIconComponent('User'),
					entityType: 'User',
					component: UserTab,
					requiredPermissions: ['Admin', 'Owner']
				},
				{
					id: entityUIConfig.UserApiKey!.tabId,
					label: TAB_LABELS[entityUIConfig.UserApiKey!.tabId],
					icon: entities.getIconComponent('UserApiKey'),
					entityType: 'UserApiKey',
					component: UserApiKeyTab,
					requiredPermissions: ['Member', 'Admin', 'Owner']
				},
				{
					id: entityUIConfig.Credential!.tabId,
					label: TAB_LABELS[entityUIConfig.Credential!.tabId],
					icon: entities.getIconComponent('Credential'),
					entityType: 'Credential',
					component: CredentialsTab
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
		const tabs: Array<{
			id: string;
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			component: any;
			isReadOnly: boolean;
			subTabIds?: string[];
			subTabDefs?: SubTab[];
			subTabNotifications?: Record<string, string>;
		}> = [];

		// Helper to extract tabs from an item and its children
		function extractTabsFromItem(item: NavItem) {
			if (item.subTabs && item.subTabs.length > 0) {
				// Emit a single entry with all sub-tab IDs and defs
				const visibleSubTabs = item.subTabs.filter(
					(st) =>
						!st.requiredPermissions ||
						st.requiredPermissions.length === 0 ||
						(userPermissions && st.requiredPermissions.includes(userPermissions))
				);
				if (visibleSubTabs.length > 0) {
					tabs.push({
						id: item.id,
						component: null,
						isReadOnly,
						subTabIds: visibleSubTabs.map((st) => st.id),
						subTabDefs: visibleSubTabs,
						subTabNotifications:
							item.id === 'discovery' && hasActiveSessions
								? { [entityUIConfig.Discovery!.tabId]: entities.getColorHelper('Discovery').rgb }
								: undefined
					});
				}
			} else if (item.component) {
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

	// n9e 嵌入模式隐藏的侧栏项/分组 id(账户/租户级壳,嵌入时不需要)。
	// 方案 B(半管理台):隐藏 Users/API Keys/Settings/Support;保留 Tags/Credentials/Scans/Daemons。
	// 顶层项按 id;整段分组按 section id。
	const EMBED_HIDDEN_IDS = new Set<string>([
		'support',
		'settings',
		entityUIConfig.User!.tabId,
		entityUIConfig.UserApiKey!.tabId
	]);
	const EMBED_HIDDEN_SECTION_IDS = new Set<string>([]);

	// Helper to check if item should be visible
	function isItemVisible(item: NavItem): boolean {
		if (isEmbed && EMBED_HIDDEN_IDS.has(item.id)) {
			return false;
		}
		return (
			hasRequiredPermissions(item) &&
			meetsBillingRequirement(item) &&
			meetsDemoModeRequirements(item)
		);
	}

	// Helper to filter an item and its children/subTabs
	function filterItemWithChildren(item: NavItem): NavItem | null {
		if (!isItemVisible(item)) {
			return null;
		}

		// If item has subTabs, filter by permissions
		if (item.subTabs) {
			const visibleSubTabs = item.subTabs.filter(
				(st) =>
					!st.requiredPermissions ||
					st.requiredPermissions.length === 0 ||
					(userPermissions && st.requiredPermissions.includes(userPermissions))
			);
			if (visibleSubTabs.length === 0) {
				return null;
			}
			return { ...item, subTabs: visibleSubTabs };
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
				// n9e 嵌入:整段隐藏的分组(如 platform)
				if (isEmbed && EMBED_HIDDEN_SECTION_IDS.has(configItem.id)) {
					return null;
				}
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

		return unsubscribeDaemon;
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

	function expandSidebar() {
		if (collapsed) {
			collapsed = false;
			if (typeof window !== 'undefined') {
				try {
					localStorage.setItem(SIDEBAR_STORAGE_KEY, JSON.stringify(false));
				} catch (error) {
					console.error('Failed to save sidebar state to localStorage:', error);
				}
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
		} else if (item.subTabs && item.subTabs.length > 0) {
			// Navigate to first visible sub-tab, or stay on current if already on one
			const isAlreadyOnSubTab = item.subTabs.some((st) => activeTab === st.id);
			if (!isAlreadyOnSubTab) {
				activeTab = item.subTabs[0].id;
			}
		} else {
			activeTab = item.id;
		}
	}

	// Helper to check if a nav item (or its sub-tabs) is currently active
	function isItemActive(item: NavItem): boolean {
		if (item.subTabs) {
			return item.subTabs.some((st) => activeTab === st.id);
		}
		return activeTab === item.id;
	}

	const inactiveButtonClass =
		'text-tertiary hover:text-secondary hover:bg-gray-100 dark:hover:bg-gray-800 border border-[var(--color-bg-sidebar)]';

	const sectionHeaderClass =
		'text-secondary hover:text-primary flex w-full items-center rounded-lg text-xs font-semibold uppercase tracking-wide transition-colors hover:bg-gray-100/50 dark:hover:bg-gray-800/50';

	const baseClasses = 'flex w-full items-center rounded-lg font-medium transition-colors';
</script>

<div
	class="sidebar flex flex-shrink-0 flex-col transition-all duration-300"
	class:w-16={collapsed}
	class:w-48={!collapsed}
>
	<!-- Logo/Brand -->
	<div class="flex min-h-0 flex-1 flex-col">
		<div class="border-b px-2 py-4" style="border-color: var(--color-border)">
			<button
				onclick={toggleCollapse}
				class="text-tertiary hover:text-secondary flex w-full items-center rounded-lg transition-colors hover:bg-gray-100 dark:hover:bg-gray-800"
				style="height: 2rem; padding: 0.375rem 0.75rem;"
				aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
			>
				<Menu class="h-5 w-5 flex-shrink-0" />
				{#if !collapsed && !isEmbed}
					<img src="/logos/scanopy-logo.png" alt="Logo" class="ml-2 h-6 w-auto rounded" />
					<h1 class="text-primary ml-1.5 text-sm font-bold">Scanopy</h1>
				{/if}
			</button>
			{#if !collapsed && isDemoOrg}
				<div class="mt-2 flex justify-center">
					<Tag label="Demo" color="Yellow" />
				</div>
			{/if}
		</div>

		<!-- Sidebar Checklist -->
		{#if showSidebarChecklist}
			<SidebarChecklist
				{onboarding}
				{collapsed}
				onNavigate={(tab) => {
					activeTab = tab;
				}}
				{isDiscoveryActive}
				{daemonStatus}
				onExpandSidebar={expandSidebar}
			/>
		{/if}

		<!-- Main Navigation -->
		<nav class="flex-1 overflow-y-auto px-2 py-2">
			<ul class="space-y-3">
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
									<span class="flex-1 text-left">{mLabel(configItem.label)}</span>
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
												class="{baseClasses} {isItemActive(item)
													? 'text-primary border border-blue-500/30 bg-blue-100 dark:border-blue-600 dark:bg-blue-700'
													: inactiveButtonClass}"
												style="height: 2rem; padding: 0.375rem 0.75rem;"
												title={collapsed ? item.label : ''}
											>
												<span class="relative">
													<item.icon class="h-4 w-4 flex-shrink-0" />
													{#if item.id === 'discovery' && hasActiveSessions}
														<span
															class="absolute -right-1 -top-1 h-2.5 w-2.5 rounded-full"
															style="background-color: {entities.getColorHelper('Discovery').rgb}"
														></span>
													{/if}
												</span>
												{#if !collapsed}
													<span class="ml-2.5 truncate">{mLabel(item.label)}</span>
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
																	? 'text-primary border border-blue-500/30 bg-blue-100 dark:border-blue-600 dark:bg-blue-700'
																	: inactiveButtonClass}"
																style="height: 2.25rem; padding: 0.375rem 0.75rem;"
																title={collapsed ? child.label : ''}
															>
																<child.icon class="h-4 w-4 flex-shrink-0" />
																{#if !collapsed}
																	<span class="ml-3 truncate text-sm">{mLabel(child.label)}</span>
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
									? 'text-primary border border-blue-500/30 bg-blue-100 dark:border-blue-600 dark:bg-blue-700'
									: inactiveButtonClass}"
								style="height: 2rem; padding: 0.375rem 0.75rem;"
								title={collapsed ? configItem.label : ''}
							>
								<configItem.icon class="h-4 w-4 flex-shrink-0" />
								{#if !collapsed}
									<span class="ml-2.5 truncate">{mLabel(configItem.label)}</span>
								{/if}
							</button>
						</li>
					{/if}
				{/each}
			</ul>
		</nav>
	</div>

	<!-- Bottom Navigation -->
	<div class="flex-shrink-0 border-t px-2 py-2" style="border-color: var(--color-border)">
		<ul class="space-y-1">
			{#snippet sidebarAmberCta({
				Icon,
				label,
				onclick
			}: {
				Icon: IconComponent;
				label: string;
				onclick: () => void;
			})}
				<button
					class="{baseClasses} text-amber-400 hover:bg-amber-500/10"
					style="height: 2rem; padding: 0.375rem 0.75rem;"
					title={collapsed ? label : ''}
					{onclick}
				>
					<span class="relative">
						<Icon class="h-5 w-5 flex-shrink-0" />
					</span>
					{#if !collapsed}
						<span class="ml-3 truncate">{label}</span>
					{/if}
				</button>
			{/snippet}

			{#if showTrialPill}
				<li>
					{@render sidebarAmberCta({
						Icon: Clock,
						label: trialPillLabel,
						onclick: () =>
							startSetupPayment({
								org: organization,
								source: 'sidebar_trial_pill',
								trialDaysLeft
							})
					})}
				</li>
			{:else if showFreeUpgradeButton}
				<li>
					{@render sidebarAmberCta({
						Icon: ArrowUpCircle,
						label: common_upgrade(),
						onclick: () => triggerUpgrade({ source: 'sidebar', surface: 'sidebar' })
					})}
				</li>
			{/if}
			{#each bottomNavItems as item (item.id)}
				{#if !isSection(item)}
					<li>
						<button
							onclick={() => handleItemClick(item)}
							class="{baseClasses} {activeTab === item.id ||
							(item.id === 'settings' && showSettings)
								? 'text-primary border border-blue-500/30 bg-blue-100 dark:border-blue-600 dark:bg-blue-700'
								: inactiveButtonClass}"
							style="height: 2rem; padding: 0.375rem 0.75rem;"
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
								<span class="ml-3 truncate">{mLabel(item.label)}</span>
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

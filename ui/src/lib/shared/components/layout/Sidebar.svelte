<script lang="ts">
	import daemonVersionGates from '$lib/data/daemon-version-gates.json';
	import { useCan, type Resource } from '$lib/features/auth/access';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive, isPlanLapsed } from '$lib/features/organizations/types';
	import SettingsModal from '$lib/features/settings/SettingsModal.svelte';
	import {
		billingPlans,
		daemonHealths,
		entities,
		productAreas,
		setupItemTiers
	} from '$lib/shared/stores/metadata';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import { useApiKeysQuery } from '$lib/features/daemon_api_keys/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { modalState } from '$lib/shared/stores/modal-registry';
	import { entityUIConfig, TAB_LABELS } from '$lib/shared/entity-ui-config';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { Menu, ChevronDown, History, Settings, Clock, Search } from '@lucide/svelte';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';
	import PaidCrown from '$lib/shared/components/PaidCrown.svelte';
	import NavBadge from '$lib/shared/components/layout/NavBadge.svelte';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { GLOBAL_SEARCH_KEY, globalSearchOpen } from '$lib/features/search/results';
	import { tableSearchFocused } from '$lib/shared/components/data/controls/listShortcuts';
	import { onMount } from 'svelte';
	import type { Component } from 'svelte';
	import type { SubTab } from '$lib/shared/components/layout/ContentSubTabs.svelte';
	import {
		common_plans,
		billing_trialPill,
		billing_trialPillOneDay,
		billing_trialPillToday,
		globalSearch_openWithShortcut,
		globalSearch_searchAll,
		globalSearch_placeholder,
		daemons_legacyKeyHelp,
		common_settings,
		common_shortcutsPausedInSearch
	} from '$lib/paraglide/messages';
	import {
		getTrialDaysLeft,
		isTrialingWithoutPayment,
		isMissingPaymentMethod
	} from '$lib/shared/utils/trial';
	import { isCloud, useConfigQuery } from '$lib/shared/stores/config-query';
	import { useDashboardQuery } from '$lib/features/home/queries';

	// Import tab components
	import TopologyTab from '$lib/features/topology/components/TopologyTab.svelte';
	import DiscoveryScheduledTab from '$lib/features/discovery/components/tabs/DiscoveryScheduledTab.svelte';
	import DiscoveryHistoryTab from '$lib/features/discovery/components/tabs/DiscoveryHistoryTab.svelte';
	import SitesTab from '$lib/features/sites/components/SitesTab.svelte';
	import SubnetTab from '$lib/features/subnets/components/SubnetTab.svelte';
	import VlanTab from '$lib/features/vlans/components/VlanTab.svelte';
	import HostTab from '$lib/features/hosts/components/HostTab.svelte';
	import ServiceTab from '$lib/features/services/components/ServiceTab.svelte';
	import DaemonTab from '$lib/features/daemons/components/DaemonTab.svelte';
	import ApiKeyTab from '$lib/features/daemon_api_keys/components/ApiKeyTab.svelte';
	import UserTab from '$lib/features/users/components/UserTab.svelte';
	import UserApiKeyTab from '$lib/features/user_api_keys/components/UserApiKeyTab.svelte';
	import TagTab from '$lib/features/tags/components/TagTab.svelte';
	import TagGroupTab from '$lib/features/tags/components/TagGroupTab.svelte';
	import CredentialsTab from '$lib/features/credentials/components/CredentialsTab.svelte';

	import IssuesTab from '$lib/features/home/components/IssuesTab.svelte';
	import SetupTab from '$lib/features/home/components/SetupTab.svelte';
	import { setupStepsLeft } from '$lib/features/home/setup-list';

	let {
		activeTab = $bindable('topology'),
		collapsed = $bindable(false),
		// eslint-disable-next-line no-useless-assignment
		allTabs = $bindable<
			Array<{
				id: string;
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				component: any;
				subTabIds?: string[];
				subTabDefs?: SubTab[];
				subTabNotifications?: Record<string, string>;
			}>
		>([]),
		showSettings = $bindable(false),
		settingsInitialTab = 'account',
		settingsDismissible = true,
		mainAppLocked = false,
		mainAppAvailable = false,
		licensedPlanPending = false,
		showLockedMainNav = false
	}: {
		activeTab?: string;
		collapsed?: boolean;
		allTabs?: Array<{
			id: string;
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			component: any;
			subTabIds?: string[];
			subTabDefs?: SubTab[];
			subTabNotifications?: Record<string, string>;
		}>;
		showSettings?: boolean;
		settingsInitialTab?: string;
		settingsDismissible?: boolean;
		/** Org is locked out of the main app (licensed self-hosted plan on a
		 * cloud server): hide main navigation. Settings stays
		 * available, with Support as one of its tabs. */
		mainAppLocked?: boolean;
		/** Config and org have loaded and the org is not locked. Main-app queries
		 * wait for this rather than for `!mainAppLocked`, which is also true
		 * before the lock state is known, and the backend rejects them with 403
		 * for a locked org. */
		mainAppAvailable?: boolean;
		/** Passed through to the Settings modal, which shows the License tab on it
		 * while the just-picked licensed plan is still in flight. */
		licensedPlanPending?: boolean;
		/** A locked org whose main navigation still shows: a licensed self-hosted
		 * plan, where each page says it needs a cloud plan. */
		showLockedMainNav?: boolean;
	} = $props();

	// TanStack Query for current user and organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const configQuery = useConfigQuery();
	// Whether this is Scanopy Cloud, the only deployment that bills. Distinct from
	// `isBillingEnabled` below, which asks whether the org's own plan is active.
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);

	// Derived values from queries
	const can = useCan();
	let isBillingEnabled = $derived(organization ? isBillingPlanActive(organization) : false);
	let planMeta = $derived(billingPlans.getMetadata(organization?.plan?.type ?? null));
	// A free tier: cloud Free (`is_free`), or self-hosted Community, the one plan that is neither
	// commercial nor billed through Stripe. Both have plans to move up to; Starter, though not
	// commercial, is a paid Stripe plan.
	let isFreePlan = $derived(
		planMeta.is_free === true ||
			(planMeta.is_commercial === false && planMeta.is_stripe_managed === false)
	);
	let isOwner = $derived(can('Billing', 'Write'));
	let trialDaysLeft = $derived(getTrialDaysLeft(organization));
	let showTrialPill = $derived(
		isOwner &&
			isBillingEnabled &&
			isTrialingWithoutPayment(organization, isCloudDeployment) &&
			trialDaysLeft !== null &&
			trialDaysLeft <= 7
	);
	let showFreeUpgradeButton = $derived(isFreePlan && isOwner && isBillingEnabled);
	let trialPillLabel = $derived.by(() => {
		if (trialDaysLeft === null) return '';
		if (trialDaysLeft <= 0) return billing_trialPillToday();
		if (trialDaysLeft === 1) return billing_trialPillOneDay();
		return billing_trialPill({ days: trialDaysLeft });
	});

	// Show notification on settings only when trialing without payment method
	// Free plan users don't need a payment method, so no dot for them.
	// Paused is intentionally excluded — the hard-gate billing modal already
	// blocks the entire app, so a sidebar dot would be noise.
	let showBillingNotification = $derived.by(() => {
		if (!organization) return false;
		// past_due stays its own clause: dunning needs attention even with a card
		// on file. isMissingPaymentMethod is the shared no-card predicate, so this
		// dot, the banner, and the BillingTab card all key off the same rule.
		// Both clauses read org rows that only Stripe webhooks write, so they go
		// stale where billing is switched off.
		if (!isCloudDeployment) return false;
		return (
			organization.plan_status === 'past_due' ||
			isMissingPaymentMethod(organization, isCloudDeployment)
		);
	});

	// Active discovery sessions — used for notification dot on sidebar and sub-tabs
	const activeSessionsQuery = useActiveSessionsQuery(() => mainAppAvailable);
	let hasActiveSessions = $derived((activeSessionsQuery.data?.length ?? 0) > 0);

	// The most severe health among daemons that aren't Healthy — drives the Daemons nav dot,
	// same mechanism as the Scans active-sessions dot. Fixture order is severity order.
	const daemonsQuery = useDaemonsQuery({ enabled: () => mainAppAvailable });
	let worstDaemonHealth = $derived.by(() => {
		const daemons = daemonsQuery.data ?? [];
		return (
			daemonHealths
				.getItems()
				.find((h) => h.id !== 'healthy' && daemons.some((d) => d.health === h.id))?.id ?? null
		);
	});

	// Legacy (unbound) daemon API keys. When none exist, the Daemon API Keys sub-tab is
	// hidden and the Daemons group collapses to a single-entity page (no tab strip).
	const daemonApiKeysQuery = useApiKeysQuery({
		enabled: () => mainAppAvailable && can('DaemonApiKey', 'Read')
	});
	let hasLegacyDaemonKeys = $derived(
		(daemonApiKeysQuery.data ?? []).some((k) => k.daemon_id == null)
	);

	// Whether a content sub-tab should be shown: permission-gated, plus data gates
	// (the Daemon API Keys sub-tab only appears while legacy keys exist).
	function isSubTabVisible(st: { id: string; requires?: Resource }): boolean {
		if (st.requires && !can(st.requires, 'Read')) return false;
		if (st.id === entityUIConfig.DaemonApiKey!.tabId && !hasLegacyDaemonKeys) return false;
		return true;
	}

	// The Issues badge counts the Issues the summary reports, which the Issues tab pages through;
	// the Setup badge counts the steps left. Each entry shows while it has any, and while it is
	// the open tab, so clearing the last one doesn't take the page away.
	const dashboardQuery = useDashboardQuery({ enabled: () => mainAppAvailable });
	let setupIssues = $derived(dashboardQuery.data?.issue_count ?? 0);
	let setupStepsRemaining = $derived(
		setupStepsLeft(
			dashboardQuery.data?.setup_items ?? [],
			organization ? isPlanLapsed(organization) : false
		)
	);

	/** Whether the Issues or Setup entry shows: while it has something in it, or is open. */
	function isHomeTabVisible(id: string): boolean {
		if (id === 'issues') return setupIssues > 0 || activeTab === 'issues';
		if (id === 'setup') return setupStepsRemaining > 0 || activeTab === 'setup';
		return true;
	}

	// Sync settings modal state from modal registry (for deep-link opens)
	$effect(() => {
		if ($modalState.name === 'settings' && !showSettings) {
			settingsInitialTab = $modalState.tab ?? 'account';
			showSettings = true;
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
		requires?: Resource; // The resource a user has to be able to read to see this item.
		requiresBilling?: boolean; // Whether this requires billing to be enabled
		children?: NavItem[]; // Nested child items (displayed indented under parent)
		subTabs?: (SubTab & { requires?: Resource })[]; // Content area sub-tabs
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
	/**
	 * The sidebar item each entity type gets inside its section. Which section it sits in, and where,
	 * comes from the entity metadata (its product area in `category`, and declaration order), the same order the global
	 * search lists its groups in. An entity with no item here (a host's ports, say) has no tab.
	 */
	const entityNavItems: Partial<Record<EntityDiscriminants, NavItem>> = {
		Topology: {
			id: entityUIConfig.Topology!.tabId,
			label: TAB_LABELS[entityUIConfig.Topology!.tabId],
			icon: entities.getIconComponent('Topology'),
			entityType: 'Topology',
			component: TopologyTab
		},
		Site: {
			id: entityUIConfig.Site!.tabId,
			label: TAB_LABELS[entityUIConfig.Site!.tabId],
			icon: entities.getIconComponent('Site'),
			entityType: 'Site',
			component: SitesTab
		},
		Vlan: {
			id: entityUIConfig.Vlan!.tabId,
			label: TAB_LABELS[entityUIConfig.Vlan!.tabId],
			icon: entities.getIconComponent('Vlan'),
			entityType: 'Vlan',
			component: VlanTab
		},
		Subnet: {
			id: entityUIConfig.Subnet!.tabId,
			label: TAB_LABELS[entityUIConfig.Subnet!.tabId],
			icon: entities.getIconComponent('Subnet'),
			entityType: 'Subnet',
			component: SubnetTab
		},
		Host: {
			id: entityUIConfig.Host!.tabId,
			label: TAB_LABELS[entityUIConfig.Host!.tabId],
			icon: entities.getIconComponent('Host'),
			entityType: 'Host',
			component: HostTab
		},
		Service: {
			id: entityUIConfig.Service!.tabId,
			label: TAB_LABELS[entityUIConfig.Service!.tabId],
			icon: entities.getIconComponent('Service'),
			entityType: 'Service',
			component: ServiceTab
		},
		Discovery: {
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
		// Legacy daemon API keys are a sub-tab of Daemons, not an item of their own.
		Daemon: {
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
					subtitle: daemons_legacyKeyHelp({
						version: daemonVersionGates.last_without_server_provisioned_identity
					}),
					icon: entities.getIconComponent('DaemonApiKey'),
					component: ApiKeyTab,
					requires: 'DaemonApiKey'
				}
			]
		},
		// Tag groups are a sub-tab of Tags: groups exist to order and scope tags.
		Tag: {
			id: 'tags-group',
			label: TAB_LABELS[entityUIConfig.Tag!.tabId],
			icon: entities.getIconComponent('Tag'),
			subTabs: [
				{
					id: entityUIConfig.Tag!.tabId,
					label: TAB_LABELS[entityUIConfig.Tag!.tabId],
					icon: entities.getIconComponent('Tag'),
					component: TagTab
				},
				{
					id: entityUIConfig.TagGroup!.tabId,
					label: TAB_LABELS[entityUIConfig.TagGroup!.tabId],
					icon: entities.getIconComponent('TagGroup'),
					component: TagGroupTab
				}
			]
		},
		User: {
			id: entityUIConfig.User!.tabId,
			label: TAB_LABELS[entityUIConfig.User!.tabId],
			icon: entities.getIconComponent('User'),
			entityType: 'User',
			component: UserTab,
			requires: 'Member'
		},
		UserApiKey: {
			id: entityUIConfig.UserApiKey!.tabId,
			label: TAB_LABELS[entityUIConfig.UserApiKey!.tabId],
			icon: entities.getIconComponent('UserApiKey'),
			entityType: 'UserApiKey',
			component: UserApiKeyTab,
			requires: 'UserApiKey'
		},
		Credential: {
			id: entityUIConfig.Credential!.tabId,
			label: TAB_LABELS[entityUIConfig.Credential!.tabId],
			icon: entities.getIconComponent('Credential'),
			entityType: 'Credential',
			component: CredentialsTab
		}
	};

	/**
	 * One section per product area that has items, in area order, items in entity order. An area
	 * with a single item (Topology) shows that item at the top level instead of a one-row section.
	 */
	const entitySections: NavConfig = productAreas
		.getItems()
		.flatMap<NavSection | NavItem>((area) => {
			const items = entities
				.getItems()
				.filter((entity) => entity.category === area.id)
				.flatMap((entity) => entityNavItems[entity.id as EntityDiscriminants] ?? []);
			if (items.length === 1) return items;
			return items.length > 0 ? [{ id: area.id, label: productAreas.getName(area.id), items }] : [];
		});

	const baseNavConfig: NavConfig = [
		{
			id: 'setup',
			label: TAB_LABELS['setup'],
			icon: setupItemTiers.getIconComponent('Setup'),
			component: SetupTab
		},
		{
			id: 'issues',
			label: TAB_LABELS['issues'],
			icon: setupItemTiers.getIconComponent('Issue'),
			component: IssuesTab
		},
		...entitySections,
		{
			id: 'settings',
			label: common_settings(),
			icon: Settings as IconComponent,
			position: 'bottom',
			onClick: async () => {
				showSettings = true;
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
			subTabIds?: string[];
			subTabDefs?: SubTab[];
			subTabNotifications?: Record<string, string>;
		}> = [];

		// Helper to extract tabs from an item and its children
		function extractTabsFromItem(item: NavItem) {
			if (item.subTabs && item.subTabs.length > 0) {
				// Emit a single entry with all sub-tab IDs and defs
				const visibleSubTabs = item.subTabs.filter(isSubTabVisible);
				if (visibleSubTabs.length > 0) {
					tabs.push({
						id: item.id,
						component: null,
						subTabIds: visibleSubTabs.map((st) => st.id),
						subTabDefs: visibleSubTabs,
						subTabNotifications:
							item.id === 'discovery' && hasActiveSessions
								? { [entityUIConfig.Discovery!.tabId]: entities.getColorHelper('Discovery').rgb }
								: undefined
					});
				}
			} else if (item.component) {
				tabs.push({ id: item.id, component: item.component });
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

	// Helper to check billing requirements
	function meetsBillingRequirement(item: NavItem): boolean {
		// If billing not required, always show
		if (!item.requiresBilling) {
			return true;
		}

		// If billing is required, check if it's enabled
		return isBillingEnabled;
	}

	// Helper to check if item should be visible
	function isItemVisible(item: NavItem): boolean {
		return (
			(!item.requires || can(item.requires, 'Read')) &&
			meetsBillingRequirement(item) &&
			isHomeTabVisible(item.id)
		);
	}

	// Helper to filter an item and its children/subTabs
	function filterItemWithChildren(item: NavItem): NavItem | null {
		if (!isItemVisible(item)) {
			return null;
		}

		// If item has subTabs, filter by permissions + data gates
		if (item.subTabs) {
			const visibleSubTabs = item.subTabs.filter(isSubTabVisible);
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

	let mainNavItems = $derived(
		mainAppLocked && !showLockedMainNav ? [] : filterByPosition(navConfig, 'main')
	);
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

	// The badge on a nav item, or null for none: one place that maps an item to its badge (Scans →
	// active sessions; Daemons → a daemon that isn't Healthy; Issues → its Issues, counted;
	// Setup → its steps left, counted; Settings → a billing action).
	function itemBadge(item: NavItem): { color: string; count?: number } | null {
		if (item.id === 'discovery' && hasActiveSessions) {
			return { color: entities.getColorHelper('Discovery').rgb };
		}
		if (item.id === 'daemons-group' && worstDaemonHealth) {
			return { color: daemonHealths.getColorHelper(worstDaemonHealth).rgb };
		}
		if (item.id === 'issues' && setupIssues > 0) {
			return { color: setupItemTiers.getColorHelper('Issue').rgb, count: setupIssues };
		}
		if (item.id === 'setup' && setupStepsRemaining > 0) {
			return { color: setupItemTiers.getColorHelper('Setup').rgb, count: setupStepsRemaining };
		}
		if (item.id === 'settings' && showBillingNotification) {
			return { color: createColorHelper('Amber').rgb };
		}
		return null;
	}

	const inactiveButtonClass =
		'text-tertiary hover:text-secondary hover:bg-gray-100 dark:hover:bg-gray-800 border border-(--color-bg-sidebar)';

	const sectionHeaderClass =
		'text-secondary hover:text-primary flex w-full items-center rounded-lg text-xs font-semibold uppercase tracking-wide transition-colors hover:bg-gray-100/50 dark:hover:bg-gray-800/50';

	const baseClasses = 'flex w-full items-center rounded-lg font-medium transition-colors';

	/** The palette's shortcut as this platform spells it. */
	const searchShortcut = GLOBAL_SEARCH_KEY;
</script>

<div
	class="sidebar flex shrink-0 flex-col transition-all duration-300"
	class:w-16={collapsed}
	class:w-48={!collapsed}
>
	<!-- Logo/Brand -->
	<div class="flex min-h-0 flex-1 flex-col overflow-y-auto">
		<div class="px-2 py-4">
			<button
				onclick={toggleCollapse}
				class="flex w-full items-center rounded-lg text-tertiary transition-colors hover:bg-gray-100 hover:text-secondary dark:hover:bg-gray-800"
				style="height: 38px; padding: 0.375rem 0.75rem;"
				aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
			>
				<Menu class="h-5 w-5 shrink-0" />
				{#if !collapsed}
					<img src="/logos/scanopy-logo.png" alt="Logo" class="ml-2 h-6 w-auto rounded-sm" />
					<h1 class="ml-1.5 text-sm font-bold text-primary">Scanopy</h1>
				{/if}
			</button>
			<!-- Global search. Collapsed, the bar shrinks to its icon and the shortcut moves to the
			     tooltip, the way every other collapsed sidebar item keeps its label. -->
			{#if mainAppAvailable && !mainAppLocked}
				<!-- Styled as the search input it opens, like every other search box: same border,
				     height and key chip. -->
				<button
					type="button"
					onclick={() => globalSearchOpen.set(true)}
					class="{collapsed
						? `${inactiveButtonClass} justify-center rounded-lg`
						: 'input-field text-muted hover:border-gray-400 dark:hover:border-gray-500'} toolbar-control mt-2 flex w-full items-center gap-2 px-3"
					title={collapsed ? globalSearch_openWithShortcut({ shortcut: searchShortcut }) : ''}
					aria-label={globalSearch_placeholder()}
					aria-keyshortcuts={searchShortcut}
				>
					<Search class="h-4 w-4 shrink-0 text-tertiary" />
					{#if !collapsed}
						<span class="flex-1 truncate text-left">{globalSearch_searchAll()}</span>
						<!-- Dims while a table search has focus: the key types there instead. -->
						<span
							class="inline-flex transition-opacity {$tableSearchFocused ? 'opacity-40' : ''}"
							title={$tableSearchFocused ? common_shortcutsPausedInSearch() : undefined}
						>
							<KbdKey key={searchShortcut} size="sm" />
						</span>
					{/if}
				</button>
			{/if}
		</div>

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
									<span class="flex-1 text-left">{configItem.label}</span>
									<ChevronDown
										class="h-4 w-4 shrink-0 transition-transform {sectionStates[configItem.id]
											? '-rotate-90'
											: ''}"
									/>
								</button>
							{/if}

							{#if !sectionStates[configItem.id] || collapsed}
								<ul class="mt-1 space-y-1" class:mt-0={collapsed}>
									{#each configItem.items as item (item.id)}
										{@const badge = itemBadge(item)}
										<li>
											<button
												onclick={() => handleItemClick(item)}
												class="{baseClasses} {isItemActive(item)
													? 'border border-blue-500/30 bg-blue-100 text-primary dark:border-blue-600 dark:bg-blue-700'
													: inactiveButtonClass}"
												style="height: 2rem; padding: 0.375rem 0.75rem;"
												title={collapsed ? item.label : ''}
											>
												<span class="relative">
													<item.icon class="h-4 w-4 shrink-0" />
													{#if badge}
														<NavBadge {...badge} class="absolute -top-1.5 -right-1.5" />
													{/if}
												</span>
												{#if !collapsed}
													<span class="ml-2.5 truncate">{item.label}</span>
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
																	? 'border border-blue-500/30 bg-blue-100 text-primary dark:border-blue-600 dark:bg-blue-700'
																	: inactiveButtonClass}"
																style="height: 2.25rem; padding: 0.375rem 0.75rem;"
																title={collapsed ? child.label : ''}
															>
																<child.icon class="h-4 w-4 shrink-0" />
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
						{@const badge = itemBadge(configItem)}
						<li>
							<button
								onclick={() => handleItemClick(configItem)}
								class="{baseClasses} {activeTab === configItem.id ||
								(configItem.id === 'settings' && showSettings)
									? 'border border-blue-500/30 bg-blue-100 text-primary dark:border-blue-600 dark:bg-blue-700'
									: inactiveButtonClass}"
								style="height: 2rem; padding: 0.375rem 0.75rem;"
								title={collapsed ? configItem.label : ''}
							>
								<span class="relative">
									<configItem.icon class="h-4 w-4 shrink-0" />
									{#if badge}
										<NavBadge {...badge} class="absolute -top-1.5 -right-1.5" />
									{/if}
								</span>
								{#if !collapsed}
									<span class="ml-2.5 truncate">{configItem.label}</span>
								{/if}
							</button>
						</li>
					{/if}
				{/each}
			</ul>
		</nav>
	</div>

	<!-- Bottom Navigation -->
	<div class="shrink-0 border-t px-2 py-2" style="border-color: var(--color-border)">
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
					class="{baseClasses} text-amber-700 hover:bg-amber-500/10 dark:text-amber-400"
					style="height: 2rem; padding: 0.375rem 0.75rem;"
					title={collapsed ? label : ''}
					{onclick}
				>
					<span class="relative">
						<Icon class="h-5 w-5 shrink-0" />
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
				<!-- The way to the plans: a nav entry like the others, marked with the paid crown. -->
				<li>
					<button
						class="{baseClasses} {inactiveButtonClass}"
						style="height: 2rem; padding: 0.375rem 0.75rem;"
						title={collapsed ? common_plans() : ''}
						onclick={() => triggerUpgrade({ source: 'sidebar', surface: 'sidebar' })}
					>
						<PaidCrown size={20} />
						{#if !collapsed}
							<span class="ml-3 truncate">{common_plans()}</span>
						{/if}
					</button>
				</li>
			{/if}
			{#each bottomNavItems as item (item.id)}
				{#if !isSection(item)}
					{@const badge = itemBadge(item)}
					<li>
						<button
							onclick={() => handleItemClick(item)}
							class="{baseClasses} {activeTab === item.id ||
							(item.id === 'settings' && showSettings)
								? 'border border-blue-500/30 bg-blue-100 text-primary dark:border-blue-600 dark:bg-blue-700'
								: inactiveButtonClass}"
							style="height: 2rem; padding: 0.375rem 0.75rem;"
							title={collapsed ? item.label : ''}
						>
							<span class="relative">
								<item.icon class="h-5 w-5 shrink-0" />
								{#if badge}
									<NavBadge {...badge} class="absolute -top-1.5 -right-1.5" />
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
	{licensedPlanPending}
/>

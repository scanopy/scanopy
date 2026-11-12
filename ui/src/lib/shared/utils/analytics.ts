import posthog from 'posthog-js';
import { queryClient, queryKeys } from '#lib/api/query-client.ts';
import type { Organization } from '#lib/features/organizations/types.ts';
import { billingPlans } from '#lib/shared/stores/metadata.ts';
import { campaignParams, getFirstTouch, getSessionEntry } from '#lib/shared/utils/first-touch.ts';

// Event queue for events that fire before PostHog loads
type QueuedEvent =
	| { type: 'capture'; event: string; properties?: Record<string, unknown> }
	| {
			type: 'identify';
			userId: string;
			traits: Record<string, unknown>;
			setOnceTraits?: Record<string, unknown>;
	  }
	| { type: 'group'; groupType: string; groupKey: string; traits: Record<string, unknown> }
	| { type: 'reset' };

let eventQueue: QueuedEvent[] = [];

/**
 * Flush queued events to PostHog.
 * Called from AppShell when PostHog finishes loading.
 */
export function flushEventQueue() {
	if (!posthog.__loaded) return;
	const queue = eventQueue;
	eventQueue = [];
	for (const item of queue) {
		switch (item.type) {
			case 'capture':
				posthog.capture(item.event, item.properties);
				break;
			case 'identify':
				posthog.identify(item.userId, item.traits, item.setOnceTraits);
				break;
			case 'group':
				posthog.group(item.groupType, item.groupKey, item.traits);
				break;
			case 'reset':
				posthog.reset();
				break;
		}
	}
}

/**
 * Check if the current organization is in demo mode.
 * Demo users should not have their data tracked.
 */
export function isDemo(): boolean {
	const org = queryClient.getQueryData<Organization | null>(queryKeys.organizations.current());
	return billingPlans.getMetadata(org?.plan?.type ?? null).is_demo === true;
}

/**
 * Track an analytics event via PostHog.
 * PostHog is already initialized in +layout.svelte, this is just a helper.
 * Once the current organization is loaded, every event carries its
 * `organization_id` and joins its `organization` group, as backend events do,
 * whether or not `identifyUser` has run yet.
 *
 * Events focused on understanding friction.
 */
export function trackEvent(event: string, properties?: Record<string, unknown>) {
	const org = queryClient.getQueryData<Organization | null>(queryKeys.organizations.current());
	if (org?.id) {
		properties = {
			...properties,
			organization_id: org.id,
			$groups: { organization: org.id }
		};
	}
	if (posthog.__loaded) {
		posthog.capture(event, properties);
	} else {
		eventQueue.push({ type: 'capture', event, properties });
	}
}

/**
 * Identify a user in PostHog.
 * Links all events to this user's profile.
 * Safe to call multiple times - PostHog deduplicates.
 * Skips identification in demo mode.
 */
export function identifyUser(
	userId: string,
	email: string,
	organization: Organization | null | undefined
) {
	if (isDemo()) return;
	const traits: Record<string, unknown> = {
		email,
		organization_id: organization?.id ?? null,
		plan_type: organization?.plan?.type ?? null,
		plan_status: organization?.plan_status ?? null,
		has_payment_method: organization?.has_payment_method ?? null
	};

	// Latest-touch attribution (set, not set-once): a returning user arriving
	// via a new campaign gets their attribution refreshed even though the
	// initial_* props below are locked to their first visit.
	const sessionEntry = getSessionEntry();
	if (sessionEntry) {
		for (const [key, value] of Object.entries(campaignParams(sessionEntry))) {
			traits[`latest_${key}`] = value;
		}
	}

	// First-touch attribution, set once per person. Custom initial_* names
	// avoid colliding with PostHog's built-in $initial_* properties.
	let setOnceTraits: Record<string, unknown> | undefined;
	const firstTouch = getFirstTouch();
	if (firstTouch) {
		setOnceTraits = {
			initial_landing_url: firstTouch.landing_url,
			initial_referrer_url: firstTouch.referrer || null,
			first_touch_ts: firstTouch.ts
		};
		for (const [key, value] of Object.entries(campaignParams(firstTouch))) {
			setOnceTraits[`initial_${key}`] = value;
		}
	}

	if (posthog.__loaded) {
		posthog.identify(userId, traits, setOnceTraits);
	} else {
		eventQueue.push({ type: 'identify', userId, traits, setOnceTraits });
	}

	// Associate user with their organization group
	if (organization?.id) {
		const groupTraits: Record<string, unknown> = {
			plan_type: organization?.plan?.type ?? null,
			plan_status: organization?.plan_status ?? null,
			name: organization?.name ?? null
		};
		if (posthog.__loaded) {
			posthog.group('organization', organization.id, groupTraits);
		} else {
			eventQueue.push({
				type: 'group',
				groupType: 'organization',
				groupKey: organization.id,
				traits: groupTraits
			});
		}
	}
}

/**
 * Reset PostHog identity on logout.
 * Unlinks future events from the user.
 */
export function resetIdentity() {
	if (posthog.__loaded) {
		posthog.reset();
	} else {
		eventQueue.push({ type: 'reset' });
	}
}

/**
 * Fire `event` at most once per browser tab session, keyed by `key`.
 * Backed by sessionStorage; survives reloads in the same tab, resets on new tabs.
 */
const ONCE_PER_SESSION_PREFIX = 'analytics_seen:';
export function trackOncePerSession(
	event: string,
	key: string,
	properties?: Record<string, unknown>
) {
	if (typeof sessionStorage === 'undefined') {
		trackEvent(event, properties);
		return;
	}
	const sessionKey = ONCE_PER_SESSION_PREFIX + key;
	if (sessionStorage.getItem(sessionKey)) return;
	sessionStorage.setItem(sessionKey, '1');
	trackEvent(event, properties);
}

/**
 * Store an event in sessionStorage to be flushed after a page redirect.
 * Use this instead of trackEvent() when a hard navigation (window.location.href)
 * follows immediately — PostHog batches capture() calls, and the redirect
 * kills the pending request before it flushes.
 */
export function storeEventForAfterRedirect(event: string, properties?: Record<string, unknown>) {
	const events = JSON.parse(sessionStorage.getItem('pendingAnalyticsEvents') || '[]');
	events.push({ event, properties });
	sessionStorage.setItem('pendingAnalyticsEvents', JSON.stringify(events));
}

/**
 * Flush events stored by storeEventForAfterRedirect().
 * Called from AppShell when PostHog finishes loading after a redirect.
 */
export function flushStoredEvents() {
	const raw = sessionStorage.getItem('pendingAnalyticsEvents');
	if (!raw) return;
	sessionStorage.removeItem('pendingAnalyticsEvents');
	const events: { event: string; properties?: Record<string, unknown> }[] = JSON.parse(raw);
	for (const { event, properties } of events) {
		trackEvent(event, properties);
	}
}

/**
 * Get PostHog distinct ID if available.
 * Safe to call even if PostHog hasn't loaded yet (e.g., with lazy loading).
 * Uses window.posthog which is set by posthog-js when initialized.
 */
export function getPosthogDistinctId(): string | null {
	if (typeof window !== 'undefined' && (window as { posthog?: typeof posthog }).posthog) {
		return (window as { posthog?: typeof posthog }).posthog?.get_distinct_id?.() ?? null;
	}
	return null;
}

import type { components } from '#lib/api/schema';

// Re-export generated types
export type DiscoveryType = components['schemas']['DiscoveryType'];
export type DiscoveryPhase = components['schemas']['DiscoveryPhase'];
export type DiscoveryTerminalReason = components['schemas']['DiscoveryTerminalReason'];
export type HostNamingFallback = components['schemas']['HostNamingFallback'];

// Session progress updates (SSE + /active-sessions). Generated, not
// hand-maintained — a hand-written copy silently omitted fields the backend
// had been publishing for a while.
export type DiscoveryUpdatePayload = components['schemas']['DiscoveryUpdatePayload'];

import type { components } from '$lib/api/schema';

// Re-export generated types
export type DiscoveryType = components['schemas']['DiscoveryType'];
export type DiscoveryPhase = components['schemas']['DiscoveryPhase'];
export type DiscoveryTerminalReason = components['schemas']['DiscoveryTerminalReason'];
export type HostNamingFallback = components['schemas']['HostNamingFallback'];

// Variant types from DiscoveryType union for type guards
export type SelfReportDiscovery = Extract<DiscoveryType, { type: 'SelfReport' }>;
export type NetworkDiscovery = Extract<DiscoveryType, { type: 'Network' }>;
export type DockerDiscovery = Extract<DiscoveryType, { type: 'Docker' }>;

// Session progress updates (SSE + /active-sessions). Generated, not
// hand-maintained — a hand-written copy silently omitted fields the backend
// had been publishing for a while.
export type DiscoveryUpdatePayload = components['schemas']['DiscoveryUpdatePayload'];

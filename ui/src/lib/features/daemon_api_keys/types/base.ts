import type { components } from '#lib/api/schema';

export type DaemonApiKey = components['schemas']['DaemonApiKey'];
// Alias for backwards compatibility within this feature
export type ApiKey = DaemonApiKey;

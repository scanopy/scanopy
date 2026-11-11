import type { components } from '#lib/api/schema';

// Re-export generated types
export type Service = components['schemas']['Service'];
export type ServiceVirtualization = components['schemas']['ServiceVirtualization'];
export type DockerVirtualization = components['schemas']['DockerVirtualization'];
export type Binding = components['schemas']['Binding'];

// Utility types for binding discrimination
export type PortBinding = Binding & { type: 'Port' };
export type IPAddressBinding = Binding & { type: 'IPAddress' };

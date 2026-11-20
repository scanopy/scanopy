/**
 * A scan's routed targets. Every scan sweeps its daemon's interfaced subnets; `subnet_ids` on a
 * scan configuration lists only the routed subnets swept on top of them.
 */
import type { Discovery } from '../types/base';

/** The live Unified scan configurations, on `siteId` when given. */
export function liveUnifiedScans(discoveries: Discovery[], siteId?: string): Discovery[] {
	return discoveries.filter(
		(d) =>
			d.run_type.type !== 'Historical' &&
			d.discovery_type.type === 'Unified' &&
			(siteId === undefined || d.site_id === siteId)
	);
}

/** The live Unified scans that sweep `subnetId` as a routed subnet. */
export function scansTargeting(subnetId: string, discoveries: Discovery[]): Discovery[] {
	return liveUnifiedScans(discoveries).filter(
		(d) => d.discovery_type.type === 'Unified' && !!d.discovery_type.subnet_ids?.includes(subnetId)
	);
}

/** `discovery` with `subnetId` added to its routed subnets. */
export function withRoutedTarget(discovery: Discovery, subnetId: string): Discovery {
	const discoveryType = discovery.discovery_type;
	if (discoveryType.type !== 'Unified' && discoveryType.type !== 'Network') return discovery;
	const current = discoveryType.subnet_ids ?? [];
	if (current.includes(subnetId)) return discovery;
	return {
		...discovery,
		discovery_type: { ...discoveryType, subnet_ids: [...current, subnetId] }
	};
}

/**
 * What a new range runs into among the subnets Scanopy already holds, for the create flow's first
 * step: a subnet it duplicates, the assumed ranges it absorbs, and the subnets it contains.
 *
 * The rules are the server's (`SubnetService::check_cidr_change` and `absorb_assumed_ranges`), read
 * from the subnets list the page already has so the flow can say what will happen before anything
 * is sent. The server still decides.
 */
import { cidrContains } from '#lib/shared/utils/cidr.ts';
import { isProvisionalCidr } from '#lib/shared/utils/cidr-source.ts';
import { isContainerSubnet } from './queries';
import type { SubnetResponse } from './types/base';

export interface NewCidrChecks {
	/** A subnet on this site with the same range. The flow edits it instead of creating another. */
	sameHere: SubnetResponse | null;
	/** Subnets on other sites with the same range. Allowed, but worth knowing. */
	sameElsewhere: SubnetResponse[];
	/** Assumed ranges on this site inside the new one, which creating it merges in. */
	assumedInside: SubnetResponse[];
	/** Settled subnets on this site inside the new one, which it will hold as nested ranges. */
	inside: SubnetResponse[];
}

/** Whether a subnet takes part: the seeded organizational rows and container bridges never do. */
function comparable(subnet: SubnetResponse): boolean {
	return !isContainerSubnet(subnet) && subnet.virtualization_service_id == null;
}

export function checkNewCidr(
	cidr: string,
	siteId: string,
	subnets: SubnetResponse[]
): NewCidrChecks {
	const candidates = subnets.filter(comparable);
	const same = candidates.filter((s) => s.cidr === cidr);
	const onSite = candidates.filter((s) => s.site_id === siteId && s.cidr !== cidr);
	const nested = onSite.filter((s) => cidrContains(cidr, s.cidr));
	return {
		sameHere: same.find((s) => s.site_id === siteId && !isProvisionalCidr(s)) ?? null,
		sameElsewhere: same.filter((s) => s.site_id !== siteId),
		assumedInside: [
			...same.filter((s) => s.site_id === siteId && isProvisionalCidr(s)),
			...nested.filter(isProvisionalCidr)
		],
		inside: nested.filter((s) => !isProvisionalCidr(s))
	};
}

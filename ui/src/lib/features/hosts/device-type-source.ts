import type { components } from '#lib/api/schema';
import type { TagProps } from '#lib/shared/components/data/types.ts';
import { attributeSourceLabel, attributeSourceTag } from '#lib/shared/utils/attribute-source.ts';
import {
	common_containers,
	common_hostname,
	common_name,
	common_operatingSystem,
	hosts_deviceTypeSource_containersTitle,
	hosts_deviceTypeSource_derived,
	hosts_deviceTypeSource_hostnameTitle,
	hosts_deviceTypeSource_macVendor,
	hosts_deviceTypeSource_macVendorTitle,
	hosts_deviceTypeSource_nameTitle,
	hosts_deviceTypeSource_noEvidence,
	hosts_deviceTypeSource_noEvidenceTitle,
	hosts_deviceTypeSource_osTitle,
	hosts_deviceTypeSource_serviceTitle
} from '#lib/paraglide/messages.js';

export type HostDeviceTypeSource = components['schemas']['HostDeviceTypeSource'];

function derived(from: string, title: string): TagProps {
	return { label: hosts_deviceTypeSource_derived({ from }), color: 'Gray', title };
}

/**
 * Where a host's device type came from, as a tag. A fingerprint match is stored with a source of
 * its own and shows it as any attributed value does. Every other rung of `Host::device_type` is
 * derived from other values each time the host is read, and the tag says which.
 */
export function deviceTypeSourceTag(source: HostDeviceTypeSource): TagProps {
	switch (source.type) {
		case 'Matched':
			return attributeSourceTag(source.source);
		case 'Service':
			return derived(
				source.service_name,
				hosts_deviceTypeSource_serviceTitle({ service: source.service_name })
			);
		case 'OperatingSystem':
			return derived(
				common_operatingSystem(),
				hosts_deviceTypeSource_osTitle({ source: attributeSourceLabel(source.source) })
			);
		case 'Hostname':
			return derived(common_hostname(), hosts_deviceTypeSource_hostnameTitle());
		case 'Name':
			return derived(common_name(), hosts_deviceTypeSource_nameTitle());
		case 'MacVendor':
			return derived(hosts_deviceTypeSource_macVendor(), hosts_deviceTypeSource_macVendorTitle());
		case 'Containers':
			return derived(common_containers(), hosts_deviceTypeSource_containersTitle());
		case 'NoEvidence':
			return {
				label: hosts_deviceTypeSource_noEvidence(),
				color: 'Gray',
				title: hosts_deviceTypeSource_noEvidenceTitle()
			};
	}
}

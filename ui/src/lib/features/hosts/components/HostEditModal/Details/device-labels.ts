/**
 * The device-label fields' form values, and what the update request carries for them.
 *
 * Text facts are sent as typed: the server keeps a value sent back unchanged at its own source and
 * records a changed one as typed by a person. The device type is different, because the form
 * shows the *derived* type, which the server would otherwise record as a choice the person made.
 * So it is sent only when the person changed it.
 */

import type { HostDeviceFacts, HostFormData } from '#lib/features/hosts/types/base.ts';
/** Registry ids of the text facts a person can set in the device facts card. */
export const EDITABLE_TEXT_FACTS = [
	'manufacturer',
	'model',
	'serial_number',
	'asset_tag',
	'sys_location',
	'sys_contact',
	'management_url'
] as const;

type TextFact = (typeof EDITABLE_TEXT_FACTS)[number];
type DeviceType = NonNullable<HostDeviceFacts['device_type']>;

export type DeviceLabelValues = Record<TextFact, string> & { device_type: DeviceType };

export function deviceLabelDefaults(host: HostFormData): DeviceLabelValues {
	const text = Object.fromEntries(
		EDITABLE_TEXT_FACTS.map((id) => [id, (host[id] as string | null | undefined) ?? ''])
	) as Record<TextFact, string>;
	return { ...text, device_type: host.device_type ?? 'Unidentified' };
}

export function deviceFactsToSubmit(
	values: DeviceLabelValues,
	host: HostFormData
): HostDeviceFacts {
	const facts: HostDeviceFacts = Object.fromEntries(
		EDITABLE_TEXT_FACTS.map((id) => [id, values[id] ?? ''])
	);
	if (values.device_type !== (host.device_type ?? 'Unidentified')) {
		facts.device_type = values.device_type;
	}
	return facts;
}

import { deviceTypes } from '#lib/shared/stores/metadata.ts';
import type { EntityDisplayComponent } from '#lib/shared/components/forms/selection/types.ts';

/** A device type id from the `device_types` fixture, as a RichSelect option. */
export type DeviceTypeOption = string;

/**
 * A device type as a row: its name, icon and colour from the `device_types` fixture, the same the
 * map and the Hosts table draw it with. Grouped by the fixture's category, so the picker groups on
 * its own once the backend gives device types one.
 */
export const DeviceTypeDisplay: EntityDisplayComponent<DeviceTypeOption, void> = {
	getId: (id) => id,
	getLabel: (id) => deviceTypes.getName(id),
	getDescription: (id) => deviceTypes.getDescription(id),
	getIcon: (id) => deviceTypes.getIconComponent(id),
	getIconColor: (id) => deviceTypes.getColorHelper(id).icon,
	getCategory: (id) => deviceTypes.getCategory(id) || null
};

/** Every device type, by name. */
export function deviceTypeOptions(): DeviceTypeOption[] {
	return deviceTypes
		.getItems()
		.map((item) => item.id)
		.sort((a, b) => deviceTypes.getName(a).localeCompare(deviceTypes.getName(b)));
}

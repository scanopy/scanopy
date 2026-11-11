import { deviceTypes } from '#lib/shared/stores/metadata.ts';
import type { IconComponent } from '#lib/shared/utils/types.ts';
import type { Host } from './types/base';
import DeviceGlyph from '#lib/features/hosts/components/DeviceGlyph.svelte';

/**
 * The icon for a specific host: its device type's glyph, in the Host colour every device type shares.
 *
 * Every surface that draws one host (a card, a chip, a hover card, a modal header) reads this, so
 * a host is the same picture in the list, the map and the editor. A host nothing identifies, or one
 * known only by id, is `Unidentified`.
 */
export function hostIcon(host: Pick<Host, 'device_type'> | null | undefined): IconComponent {
	return deviceTypes.getIconComponent(host?.device_type ?? 'Unidentified');
}

/**
 * The same icon as a tile: the device type's glyph on its neutral square, for a host's card (the hover
 * card, a picker row, the inspector), where the icon sits in its own slot. Chips and inline
 * mentions use `hostIcon`, bare: `HostDisplay.getChipIcon` gives it to every host `EntityTag`.
 */
export function hostTile(host: Pick<Host, 'device_type'> | null | undefined): IconComponent {
	const deviceType = host?.device_type ?? 'Unidentified';
	// Pre-binds the device type, as `createLogoIconComponent` pre-binds a logo. The slot's own sizing
	// classes are dropped: the tile has a fixed size and draws its glyph in the text colour.
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	return ($$payload: any) => DeviceGlyph($$payload, { deviceType, size: 'md' });
}

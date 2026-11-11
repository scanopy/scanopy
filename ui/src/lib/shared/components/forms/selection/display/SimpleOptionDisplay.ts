import type { TagProps } from '../../../data/types';
import type { IconComponent } from '#lib/shared/utils/types.ts';
import type { EntityDisplayComponent } from '../types';

/**
 * A simple option for use with RichSelect when you just need
 * text labels, optional descriptions, optional tags (e.g. "Upgrade"),
 * and optional disabled state — without a full entity model.
 */
export interface SimpleOption {
	value: string;
	label: string;
	disabled?: boolean;
	disabledReason?: string;
	description?: string;
	tags?: TagProps[];
	icon?: IconComponent;
	iconColor?: string;
}

export const SimpleOptionDisplay: EntityDisplayComponent<SimpleOption, void> = {
	getId: (item) => item.value,
	getLabel: (item) => item.label,
	getDescription: (item) => item.description ?? '',
	getDisabled: (item) => item.disabled ?? false,
	getDisabledReason: (item) => item.disabledReason ?? null,
	getTags: (item) => item.tags ?? [],
	getIcon: (item) => item.icon ?? null,
	getIconColor: (item) => item.iconColor ?? null
};

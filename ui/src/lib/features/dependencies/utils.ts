import type { DependencyType } from './types/base';
import { edgeTypes } from '#lib/shared/stores/metadata.ts';
import { AVAILABLE_COLORS, type Color } from '#lib/shared/utils/styling.ts';

type NameGenerator = (names: string[]) => string;

const nameGenerators: Record<DependencyType, NameGenerator> = {
	RequestPath: (names) => {
		if (names.length === 0) return '';
		if (names.length === 1) return names[0];
		return `${names[0]} → ${names[names.length - 1]}`;
	},
	HubAndSpoke: (names) => {
		if (names.length === 0) return '';
		return `${names[0]} Hub`;
	}
};

/**
 * Generate a default dependency name from the selected element names.
 * @param type - The dependency type
 * @param names - Ordered display names of the selected elements
 */
export function generateDependencyName(type: DependencyType, names: string[]): string {
	return nameGenerators[type](names);
}

/**
 * The colours a dependency edge can take: the palette less every colour a non-dependency edge
 * type draws in, so a dependency never reads as a host, container or physical-link edge.
 */
export function dependencyColors(): Color[] {
	const claimed = new Set(
		edgeTypes
			.getItems()
			.filter((type) => !type.metadata.is_dependency_edge)
			.map((type) => type.color)
	);
	return AVAILABLE_COLORS.filter((color) => !claimed.has(color));
}

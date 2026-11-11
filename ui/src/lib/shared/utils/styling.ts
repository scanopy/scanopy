import * as LucideIcons from '@lucide/svelte';
import type { IconComponent } from './types';
import colors from 'tailwindcss/colors';
import LogoIcon from '#lib/shared/components/data/LogoIcon.svelte';
import type { LogoTheming } from '#lib/shared/utils/logos.ts';
import type { components } from '#lib/api/schema';

// Use the generated Color type from OpenAPI schema
export type Color = components['schemas']['Color'];

export interface ColorStyle {
	text: string;
	bg: string;
	border: string;
	icon: string;
	ring: string;
	stroke: string;
	color: Color;
	rgb: string; // Added RGB value
}

// Unified color map using dark: variants — Tailwind handles light/dark switching automatically.
// The rgb values use the -500 shade as a middle ground visible on both light and dark backgrounds.
export const COLOR_MAP: Record<Color, ColorStyle> = {
	Pink: {
		color: 'Pink',
		text: 'text-pink-800 dark:text-pink-400',
		bg: 'bg-pink-200 border-pink-400 dark:bg-pink-900/50 dark:border-pink-600',
		border: 'border-pink-400 dark:border-pink-600',
		icon: 'text-pink-600 dark:text-pink-400',
		ring: 'ring-pink-400',
		stroke: 'stroke-pink-600 dark:stroke-pink-400',
		rgb: 'rgb(236, 72, 153)' // pink-500
	},
	Rose: {
		color: 'Rose',
		text: 'text-rose-800 dark:text-rose-400',
		bg: 'bg-rose-200 border-rose-400 dark:bg-rose-900/50 dark:border-rose-600',
		border: 'border-rose-400 dark:border-rose-600',
		icon: 'text-rose-600 dark:text-rose-400',
		ring: 'ring-rose-400',
		stroke: 'stroke-rose-600 dark:stroke-rose-400',
		rgb: 'rgb(244, 63, 94)' // rose-500
	},
	Red: {
		color: 'Red',
		text: 'text-red-800 dark:text-red-400',
		bg: 'bg-red-200 border-red-400 dark:bg-red-900/50 dark:border-red-600',
		border: 'border-red-400 dark:border-red-600',
		icon: 'text-red-600 dark:text-red-400',
		ring: 'ring-red-400',
		stroke: 'stroke-red-600 dark:stroke-red-400',
		rgb: 'rgb(239, 68, 68)' // red-500
	},
	Amber: {
		color: 'Amber',
		text: 'text-amber-800 dark:text-amber-400',
		bg: 'bg-amber-200 border-amber-400 dark:bg-amber-900/50 dark:border-amber-600',
		border: 'border-amber-400 dark:border-amber-600',
		icon: 'text-amber-600 dark:text-amber-400',
		ring: 'ring-amber-400',
		stroke: 'stroke-amber-600 dark:stroke-amber-400',
		rgb: 'rgb(245, 158, 11)' // amber-500
	},
	Orange: {
		color: 'Orange',
		text: 'text-orange-800 dark:text-orange-400',
		bg: 'bg-orange-200 border-orange-400 dark:bg-orange-900/50 dark:border-orange-600',
		border: 'border-orange-400 dark:border-orange-600',
		icon: 'text-orange-600 dark:text-orange-400',
		ring: 'ring-orange-400',
		stroke: 'stroke-orange-600 dark:stroke-orange-400',
		rgb: 'rgb(249, 115, 22)' // orange-500
	},
	Yellow: {
		color: 'Yellow',
		text: 'text-yellow-800 dark:text-yellow-400',
		bg: 'bg-yellow-200 border-yellow-400 dark:bg-yellow-900/50 dark:border-yellow-600',
		border: 'border-yellow-400 dark:border-yellow-600',
		icon: 'text-yellow-600 dark:text-yellow-400',
		ring: 'ring-yellow-400',
		stroke: 'stroke-yellow-600 dark:stroke-yellow-400',
		rgb: 'rgb(234, 179, 8)' // yellow-500
	},
	Green: {
		color: 'Green',
		text: 'text-green-800 dark:text-green-400',
		bg: 'bg-green-200 border-green-400 dark:bg-green-900/50 dark:border-green-600',
		border: 'border-green-400 dark:border-green-600',
		icon: 'text-green-600 dark:text-green-400',
		ring: 'ring-green-400',
		stroke: 'stroke-green-600 dark:stroke-green-400',
		rgb: 'rgb(34, 197, 94)' // green-500
	},
	Emerald: {
		color: 'Emerald',
		text: 'text-emerald-800 dark:text-emerald-400',
		bg: 'bg-emerald-200 border-emerald-400 dark:bg-emerald-900/50 dark:border-emerald-600',
		border: 'border-emerald-400 dark:border-emerald-600',
		icon: 'text-emerald-600 dark:text-emerald-400',
		ring: 'ring-emerald-400',
		stroke: 'stroke-emerald-600 dark:stroke-emerald-400',
		rgb: 'rgb(16, 185, 129)' // emerald-500
	},
	Teal: {
		color: 'Teal',
		text: 'text-teal-800 dark:text-teal-400',
		bg: 'bg-teal-200 border-teal-400 dark:bg-teal-900/50 dark:border-teal-600',
		border: 'border-teal-400 dark:border-teal-600',
		icon: 'text-teal-600 dark:text-teal-400',
		ring: 'ring-teal-400',
		stroke: 'stroke-teal-600 dark:stroke-teal-400',
		rgb: 'rgb(20, 184, 166)' // teal-500
	},
	Cyan: {
		color: 'Cyan',
		text: 'text-cyan-800 dark:text-cyan-400',
		bg: 'bg-cyan-200 border-cyan-400 dark:bg-cyan-900/50 dark:border-cyan-600',
		border: 'border-cyan-400 dark:border-cyan-600',
		icon: 'text-cyan-600 dark:text-cyan-400',
		ring: 'ring-cyan-400',
		stroke: 'stroke-cyan-600 dark:stroke-cyan-400',
		rgb: 'rgb(6, 182, 212)' // cyan-500
	},
	Blue: {
		color: 'Blue',
		text: 'text-blue-800 dark:text-blue-400',
		bg: 'bg-blue-200 border-blue-400 dark:bg-blue-900/50 dark:border-blue-600',
		border: 'border-blue-400 dark:border-blue-600',
		icon: 'text-blue-600 dark:text-blue-400',
		ring: 'ring-blue-400',
		stroke: 'stroke-blue-600 dark:stroke-blue-400',
		rgb: 'rgb(59, 130, 246)' // blue-500
	},
	Indigo: {
		color: 'Indigo',
		text: 'text-indigo-800 dark:text-indigo-400',
		bg: 'bg-indigo-200 border-indigo-400 dark:bg-indigo-900/50 dark:border-indigo-600',
		border: 'border-indigo-400 dark:border-indigo-600',
		icon: 'text-indigo-600 dark:text-indigo-400',
		ring: 'ring-indigo-400',
		stroke: 'stroke-indigo-600 dark:stroke-indigo-400',
		rgb: 'rgb(99, 102, 241)' // indigo-500
	},
	Purple: {
		color: 'Purple',
		text: 'text-purple-800 dark:text-purple-400',
		bg: 'bg-purple-200 border-purple-400 dark:bg-purple-900/50 dark:border-purple-600',
		border: 'border-purple-400 dark:border-purple-600',
		icon: 'text-purple-600 dark:text-purple-400',
		ring: 'ring-purple-400',
		stroke: 'stroke-purple-600 dark:stroke-purple-400',
		rgb: 'rgb(168, 85, 247)' // purple-500
	},
	Fuchsia: {
		color: 'Fuchsia',
		text: 'text-fuchsia-800 dark:text-fuchsia-400',
		bg: 'bg-fuchsia-200 border-fuchsia-400 dark:bg-fuchsia-900/50 dark:border-fuchsia-600',
		border: 'border-fuchsia-400 dark:border-fuchsia-600',
		icon: 'text-fuchsia-600 dark:text-fuchsia-400',
		ring: 'ring-fuchsia-400',
		stroke: 'stroke-fuchsia-600 dark:stroke-fuchsia-400',
		rgb: 'rgb(217, 70, 239)' // fuchsia-500
	},
	Violet: {
		color: 'Violet',
		text: 'text-violet-800 dark:text-violet-400',
		bg: 'bg-violet-200 border-violet-400 dark:bg-violet-900/50 dark:border-violet-600',
		border: 'border-violet-400 dark:border-violet-600',
		icon: 'text-violet-600 dark:text-violet-400',
		ring: 'ring-violet-400',
		stroke: 'stroke-violet-600 dark:stroke-violet-400',
		rgb: 'rgb(139, 92, 246)' // violet-500
	},
	Sky: {
		color: 'Sky',
		text: 'text-sky-800 dark:text-sky-400',
		bg: 'bg-sky-200 border-sky-400 dark:bg-sky-900/50 dark:border-sky-600',
		border: 'border-sky-400 dark:border-sky-600',
		icon: 'text-sky-600 dark:text-sky-400',
		ring: 'ring-sky-400',
		stroke: 'stroke-sky-600 dark:stroke-sky-400',
		rgb: 'rgb(14, 165, 233)' // sky-500
	},
	Gray: {
		color: 'Gray',
		text: 'text-gray-800 dark:text-gray-400',
		bg: 'bg-gray-200 border-gray-300 dark:bg-gray-900/50 dark:border-gray-600',
		border: 'border-gray-400 dark:border-gray-600',
		icon: 'text-gray-600 dark:text-gray-400',
		ring: 'ring-gray-400',
		stroke: 'stroke-gray-600 dark:stroke-gray-400',
		rgb: 'rgb(107, 114, 128)' // gray-500
	},
	Lime: {
		color: 'Lime',
		text: 'text-lime-800 dark:text-lime-400',
		bg: 'bg-lime-200 border-lime-400 dark:bg-lime-900/50 dark:border-lime-600',
		border: 'border-lime-400 dark:border-lime-600',
		icon: 'text-lime-600 dark:text-lime-400',
		ring: 'ring-lime-400',
		stroke: 'stroke-lime-600 dark:stroke-lime-400',
		rgb: 'rgb(132, 204, 22)' // lime-500
	}
};

// Export available colors array derived from COLOR_MAP keys
export const AVAILABLE_COLORS = Object.keys(COLOR_MAP) as Color[];

// Convert a string to a validated Color, with fallback to 'gray'
export function toColor(value: string | null | undefined): Color {
	if (!value) return 'Gray';
	else {
		const upperValue = value.charAt(0).toUpperCase() + value.slice(1);
		return AVAILABLE_COLORS.includes(upperValue as Color) ? (upperValue as Color) : 'Gray';
	}
}

// Unified color helper - uses dark: variants so Tailwind handles theme switching
export function createColorHelper(colorName: Color | null): ColorStyle {
	const color = colorName && COLOR_MAP[colorName] ? colorName : 'Gray';
	return COLOR_MAP[color];
}

// Icon helper that converts string to component
export function createIconComponent(iconName: string | null): IconComponent {
	if (!iconName || iconName == null) return LucideIcons.HelpCircle;

	// Convert kebab-case to PascalCase for Lucide component names
	const componentName = iconName
		.split('-')
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
		.join('');

	// Return the component or fallback
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	return (LucideIcons as any)[componentName] || LucideIcons.HelpCircle;
}

// Icon helper that turns a string into an SVG
export function createLogoIconComponent(
	iconName: string | null,
	iconUrl: string,
	{
		fill = null,
		round = false,
		theming = 'Fixed',
		darkUrl = null,
		markup = null,
		framed = true
	}: {
		fill?: string | null;
		round?: boolean;
		theming?: LogoTheming;
		darkUrl?: string | null;
		markup?: string | null;
		framed?: boolean;
	} = {}
): IconComponent {
	if (!iconName || iconName == null) return LucideIcons.HelpCircle;

	// Create a wrapper component that pre-binds the iconName
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const BoundLogoIcon = ($$payload: any, $$props: Omit<any, 'iconName'>) => {
		return LogoIcon($$payload, {
			iconName,
			iconUrl,
			fill,
			round,
			theming,
			darkUrl,
			markup,
			framed,
			...$$props
		});
	};

	return BoundLogoIcon;
}

// Convenience wrapper that returns both color and icon
export function createStyle(color: Color | null, icon: string | null) {
	return {
		colors: createColorHelper(color),
		IconComponent: createIconComponent(icon),
		iconName: icon
	};
}

/**
 * Converts a Tailwind color string (e.g. "text-blue-400", "bg-blue-900/50", "blue-500")
 * to an rgba() string with optional alpha override.
 */
export function twColorToRgba(twColor: string, alphaOverride?: number): string {
	const match = twColor.match(/([a-zA-Z]+)-(\d{2,3})(?:\/(\d{1,3}))?/);
	if (!match) return 'rgba(0,0,0,0)';

	const [, colorName, shade, opacityRaw] = match;

	const palette = (colors as unknown as Record<string, Record<number, string>>)[colorName];
	if (!palette) return 'rgba(0,0,0,0)';

	const hex = palette[parseInt(shade)];
	if (!hex) return 'rgba(0,0,0,0)';

	const alpha =
		typeof alphaOverride === 'number'
			? alphaOverride
			: opacityRaw
				? parseInt(opacityRaw, 10) / 100
				: 1;

	return hexToRgba(hex, alpha);
}

function hexToRgba(hex: string, alpha = 1): string {
	const cleanHex = hex.replace('#', '');
	const bigint = parseInt(cleanHex, 16);
	const r = (bigint >> 16) & 255;
	const g = (bigint >> 8) & 255;
	const b = bigint & 255;
	return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

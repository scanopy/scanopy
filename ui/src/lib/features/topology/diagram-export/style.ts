/**
 * Every size and colour the diagram exports draw with. The serializers hold none of their own.
 *
 * A value the canvas keeps as a TS constant is imported. A value the canvas sets as a Tailwind
 * class or an `app.css` token has one named constant here, with the class or token it mirrors.
 * Colours named by Tailwind classes are read from Tailwind's own palette, so a pill here is the
 * shade `COLOR_MAP` gives the same pill on the canvas.
 */

import tailwind from 'tailwindcss/colors';
import { COLOR_MAP, type Color } from '$lib/shared/utils/styling';
import { ICON_BADGE_PX, ICON_TILE_PX } from '../pipeline/build-flow-nodes';

export type DiagramTheme = 'light' | 'dark';

type Shades = Record<string, string>;

/** `#rrggbb` for a CSS `oklch(L% C H)` colour, the form Tailwind's palette uses. */
function oklchToHex(value: string): string | null {
	const match = value.match(/^oklch\(\s*([\d.]+)%\s+([\d.]+)\s+([\d.]+)/);
	if (!match) return null;
	const lightness = Number(match[1]) / 100;
	const chroma = Number(match[2]);
	const hue = (Number(match[3]) * Math.PI) / 180;
	const a = chroma * Math.cos(hue);
	const b = chroma * Math.sin(hue);
	const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
	const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
	const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;
	const linear = [
		4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
		-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
		-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s
	];
	return `#${linear
		.map((channel) => {
			const clamped = Math.min(1, Math.max(0, channel));
			const srgb = clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * clamped ** (1 / 2.4) - 0.055;
			return Math.round(srgb * 255)
				.toString(16)
				.padStart(2, '0');
		})
		.join('')}`;
}

/** A Tailwind palette colour as hex, e.g. `tailwindHex('gray', 200)`. */
export function tailwindHex(name: string, shade: number | string): string | null {
	const shades = (tailwind as unknown as Record<string, Shades | string>)[name];
	const value = typeof shades === 'object' ? shades[String(shade)] : undefined;
	if (!value) return null;
	return value.startsWith('#') ? value : oklchToHex(value);
}

function channels(hex: string): [number, number, number] {
	return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as [number, number, number];
}

/** `foreground` at `alpha` over `backdrop`, as hex. Visio has no alpha on fills. */
export function blend(foreground: string, alpha: number, backdrop: string): string {
	const top = channels(foreground);
	const under = channels(backdrop);
	return `#${top
		.map((c, i) =>
			Math.round(c * alpha + under[i] * (1 - alpha))
				.toString(16)
				.padStart(2, '0')
		)
		.join('')}`;
}

/**
 * The colour a class list gives one utility in one theme: `bg` of
 * `bg-pink-200 dark:bg-pink-900/50` is pink-200 in light, and pink-900 at 50% over `backdrop` in
 * dark. Null when the list names no palette colour for it.
 */
export function classColor(
	classes: string,
	utility: 'bg' | 'text',
	theme: DiagramTheme,
	backdrop: string
): string | null {
	for (const token of classes.split(/\s+/)) {
		const isDark = token.startsWith('dark:');
		if (isDark !== (theme === 'dark')) continue;
		const bare = isDark ? token.slice(5) : token;
		if (!bare.startsWith(`${utility}-`)) continue;
		const [name, shade, alpha] = bare.slice(utility.length + 1).split(/[-/]/);
		if (!shade) continue;
		const hex = tailwindHex(name, shade);
		if (!hex) continue;
		return alpha ? blend(hex, Number(alpha) / 100, backdrop) : hex;
	}
	return null;
}

const gray = (shade: number) => tailwindHex('gray', shade) ?? '#808080';

export interface DiagramPalette {
	/** `--color-topology-bg` */
	canvas: string;
	/** `--color-topology-node-bg` */
	node: string;
	/** `--color-topology-subgroup-bg`, pre-blended over `node`. */
	subgroup: string;
	/** The card border the raster exports draw (`topology-viewer.css`, `.hide-for-export .card`). */
	cardBorder: string;
	/** `--color-border` */
	border: string;
	/** `--color-text-primary` */
	text: string;
	/** `--color-text-secondary` */
	textSecondary: string;
	/** `--color-text-tertiary` */
	textTertiary: string;
	/** A device or port tile: `bg-gray-200 dark:bg-gray-700`. */
	tile: string;
	/** A round frame's ring and a dashed box's border: `border-gray-300 dark:border-gray-600`. */
	ring: string;
}

export const DIAGRAM_PALETTE: Record<DiagramTheme, DiagramPalette> = {
	light: {
		canvas: '#f8fafc',
		node: '#ffffff',
		subgroup: blend('#000000', 0.03, '#ffffff'),
		cardBorder: '#94a3b8',
		border: '#e2e8f0',
		text: '#0f172a',
		textSecondary: '#334155',
		textTertiary: '#64748b',
		tile: gray(200),
		ring: gray(300)
	},
	dark: {
		canvas: '#15131e',
		node: '#1a1d29',
		subgroup: blend('#ffffff', 0.04, '#1a1d29'),
		cardBorder: '#6b7280',
		border: '#374151',
		text: '#ffffff',
		textSecondary: '#d1d5db',
		textTertiary: '#9ca3af',
		tile: gray(700),
		ring: gray(600)
	}
};

/** A `Tag`'s fill and text colour for one of the app's colours, as `Tag.svelte` draws it. */
export function tagColors(
	color: Color,
	theme: DiagramTheme,
	backdrop: string
): { fill: string; text: string } {
	const style = COLOR_MAP[color] ?? COLOR_MAP.Gray;
	return {
		fill: classColor(style.bg, 'bg', theme, backdrop) ?? DIAGRAM_PALETTE[theme].tile,
		text: classColor(style.text, 'text', theme, backdrop) ?? DIAGRAM_PALETTE[theme].text
	};
}

/** Sizes in flow units (CSS px at zoom 1). */
export const STYLE = {
	/** `text-xs` */
	text: 12,
	/** `text-[11px]`: a manager group's rows. */
	rowText: 11,
	/** `text-base`: a collapsed root container's summary. */
	summaryText: 16,
	/** `leading-tight` on `text-xs` */
	lineHeight: 15,
	/** Average glyph advance of the UI font, as a fraction of the font size. */
	charWidth: 0.56,

	card: {
		/** `.card` */
		radius: 16,
		/** `px-2 py-2` */
		padding: 8,
		/** `gap-1.5` between the tile, the status pill, the label and each group box. */
		gap: 6,
		/** `.hide-for-export .card` */
		borderWidth: 1.5
	},
	tile: {
		size: ICON_TILE_PX,
		/** `my-1.5`: room for the badges that overhang the tile's corners. */
		margin: 6,
		/** `rounded-lg` */
		radius: 8,
		/** `border-2` */
		borderWidth: 2,
		/** A lucide glyph in the tile. */
		glyph: 28,
		/** A logo in the round frame's inner size. */
		frameInner: 44
	},
	status: {
		/** `h-4 w-4` ring, centred on the tile's top-left corner. */
		ring: 16,
		/** `.card-status-dot::before` */
		dot: 10,
		/** A group member row's dot: `h-2 w-2`. */
		rowDot: 8
	},
	badge: {
		size: ICON_BADGE_PX,
		serviceGlyph: 17,
		osGlyph: 15,
		frameInner: 22
	},
	/** A `Tag`: `px-2 py-0.5 gap-1`, a 16px icon, `text-xs`. */
	tag: {
		height: 20,
		paddingX: 8,
		/** `px-1.5` on a tag that is its icon alone. */
		iconOnlyPaddingX: 6,
		icon: 16,
		gap: 4,
		/** `rounded-sm` on a tag that is not a pill. */
		radius: 4
	},
	/** A manager group box on a guest's card. */
	group: {
		/** `rounded-md` */
		radius: 6,
		/** `p-1.5` */
		padding: 6,
		/** `gap-y-1.5` */
		rowGap: 6,
		/** `gap-x-1.5` */
		columnGap: 6,
		/** The glyph column, `1.25rem`, and a member row's height. */
		glyph: 20,
		/** The heading's bare logo, and the heading row's height. */
		headingGlyph: 16,
		/** The trailing column, `0.75rem`. */
		trailing: 12,
		/** A small device tile's glyph: `h-3.5 w-3.5`. */
		memberGlyph: 14
	},
	container: {
		/** `rounded-xl` */
		radius: 12,
		/** `border-top: 2px solid` in the container's colour. */
		accentHeight: 2,
		/** `rounded-lg` */
		subRadius: 8,
		/** `top-2 left-2` plus `py-0.5`: where an inline title starts. */
		inlineInset: 8,
		/** `mb-2`: an entity title pill's gap above the box. */
		entityTitleGap: 8,
		/** `py-1` around a 20px icon, in a 1px border: a grouping title pill. */
		groupTitleHeight: 30,
		groupTitleIcon: 20,
		/** `px-3 py-2`: a collapsed subcontainer's row. */
		collapsedPaddingX: 12,
		/** `px-6 py-4 gap-2`: a collapsed root container's summary block. */
		summaryPaddingY: 16,
		summaryGap: 8,
		summaryLine: 24,
		/** `px-2 py-1` around `text-xs`: a subgroup row. */
		subgroupRowHeight: 26,
		subgroupRowPaddingX: 8,
		/** A grouping subcontainer's type icon: `h-3.5 w-3.5`. */
		inlineIcon: 14
	},
	edge: {
		/** `CustomEdge`'s label card. */
		labelText: 12
	},
	header: { titleText: 18, subtitleText: 12 }
} as const;

/** A string's drawn width at a font size. The exports cannot measure text, so this estimates. */
export function textWidth(value: string, size: number): number {
	return Math.ceil(value.length * size * STYLE.charWidth);
}

/** `value` cut to fit `maxWidth` with an ellipsis, as CSS `truncate` cuts it on the canvas. */
export function truncateText(value: string, size: number, maxWidth: number): string {
	if (textWidth(value, size) <= maxWidth) return value;
	const fits = Math.max(Math.floor(maxWidth / (size * STYLE.charWidth)) - 1, 1);
	return `${value.slice(0, fits).trimEnd()}…`;
}

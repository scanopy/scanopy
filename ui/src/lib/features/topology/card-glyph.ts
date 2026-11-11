/**
 * Icons drawn as one element on topology cards.
 *
 * A lucide icon renders as an `<svg>` with two to five shape children, and a logo as a wrapper
 * `<div>` around an `<img>`. On a few thousand mounted cards that is most of the card's DOM, and
 * Blink keeps style and layout objects for each element whether or not it changes. A card instead
 * draws each icon on one element's `::before`: a mask filled with the text colour for a monochrome
 * icon, a background image for a logo (`topology-viewer.css`, `.card-glyph`).
 *
 * The source is whatever the app's icon component renders, read once per icon and kept: the
 * component is mounted into a detached element and its output inspected. Device, service and OS
 * icons all come through their existing components, so nothing here names an icon.
 */

import { mount, unmount, type Component } from 'svelte';
import type { IconComponent } from '$lib/shared/utils/types';

export type CardGlyph =
	| { type: 'mask'; key: string; svg: string; css: string }
	| {
			type: 'image';
			key: string;
			src: string;
			/** The colour that fills the frame behind the logo (`logos.json` `Solid`), if any. */
			fill: string | null;
			/** The logo is a disc, drawn at the frame's size (`logos.json` `Round`). */
			round: boolean;
			css: string;
			/** A `Paired` logo's dark-theme file. */
			darkSrc: string | null;
			darkCss: string | null;
	  };

const glyphs = new Map<string, CardGlyph>();

/** `url(...)` for a CSS value: quoted, with the characters that would end the string escaped. */
function cssUrl(url: string): string {
	return `url("${url.replace(/["\\\n]/g, (c) => `\\${c}`)}")`;
}

function svgDataUri(svg: string): string {
	return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

/**
 * The glyph for an icon component, under a key naming the icon (a device type, a service
 * definition, an OS family): the key is what keeps one read per icon.
 */
export function cardGlyph(key: string, Icon: IconComponent): CardGlyph {
	const known = glyphs.get(key);
	if (known) return known;

	const target = document.createElement('div');
	const instance = mount(Icon as unknown as Component<{ size: number }>, {
		target,
		props: { size: 24 }
	});
	const img = target.querySelector('img:not([data-logo-dark])');
	const darkSrc = target.querySelector('img[data-logo-dark]')?.getAttribute('src') ?? null;
	const svg = target.querySelector('svg');
	let glyph: CardGlyph;
	if (img) {
		const src = img.getAttribute('src') ?? '';
		glyph = {
			type: 'image',
			key,
			src,
			// `LogoIcon` marks what fills the logo's frame.
			fill: target.querySelector<HTMLElement>('[data-logo-fill]')?.dataset.logoFill ?? null,
			round: !!target.querySelector('[data-logo-round]'),
			css: cssUrl(src),
			darkSrc,
			darkCss: darkSrc ? cssUrl(darkSrc) : null
		};
	} else if (svg) {
		svg.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
		const markup = svg.outerHTML;
		// An icon drawn in `currentColor` takes the card's text colour, as a mask. One that names
		// its own colours keeps them, as an image.
		glyph = markup.includes('currentColor')
			? { type: 'mask', key, svg: markup, css: cssUrl(svgDataUri(markup)) }
			: {
					type: 'image',
					key,
					src: svgDataUri(markup),
					fill: null,
					round: false,
					css: cssUrl(svgDataUri(markup)),
					darkSrc: null,
					darkCss: null
				};
	} else {
		glyph = { type: 'mask', key, svg: '', css: 'none' };
	}
	unmount(instance);
	glyphs.set(key, glyph);
	return glyph;
}

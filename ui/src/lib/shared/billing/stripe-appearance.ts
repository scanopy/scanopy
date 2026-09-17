import type { Appearance } from '@stripe/stripe-js';

// Stripe Elements lives in an iframe, so it can't inherit the app's CSS.
// Mirror Scanopy's design tokens (read live from :root, so it tracks the
// active light/dark theme) into the Elements appearance API.
function cssVar(name: string): string {
	return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/** Elements appearance matching the app's current theme. */
export function buildStripeAppearance(): Appearance {
	const isDark = document.documentElement.classList.contains('dark');
	const accent = '#3b82f6'; // blue-500, matches btn-primary / focus ring
	const inputBg = cssVar('--color-bg-input');
	const inputBorder = cssVar('--color-border-input');
	const textPrimary = cssVar('--color-text-primary');
	return {
		theme: isDark ? 'night' : 'stripe',
		variables: {
			colorPrimary: accent,
			colorBackground: inputBg,
			colorText: textPrimary,
			colorTextSecondary: cssVar('--color-text-secondary'),
			colorTextPlaceholder: cssVar('--color-text-muted'),
			colorDanger: '#ef4444', // red-500
			fontFamily: getComputedStyle(document.body).fontFamily,
			borderRadius: '6px'
		},
		rules: {
			'.Input': {
				backgroundColor: inputBg,
				borderColor: inputBorder,
				color: textPrimary
			},
			'.Input:focus': {
				borderColor: accent,
				boxShadow: '0 0 0 2px rgba(59, 130, 246, 0.5)'
			},
			'.Tab, .AccordionItem': {
				backgroundColor: cssVar('--color-bg-elevated'),
				borderColor: cssVar('--color-border')
			},
			'.Tab:hover, .AccordionItem:hover': {
				backgroundColor: inputBg
			},
			'.Label': {
				color: cssVar('--color-text-secondary')
			}
		}
	};
}

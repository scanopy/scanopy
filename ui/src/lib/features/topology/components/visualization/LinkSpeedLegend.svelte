<script lang="ts">
	import { Panel } from '@xyflow/svelte';
	import { activeView, topologyOptions } from '../../queries';
	import { isExporting } from '../../interactions';
	import {
		effectiveEdgeColoring,
		LINK_SPEED_TIERS,
		TIER_STROKE_WIDTHS,
		UNKNOWN_STROKE_WIDTH,
		useShareEdgeColoring
	} from '../../edge-speed';
	import { LINK_SPEED_RAMP, LINK_SPEED_UNKNOWN } from '#lib/shared/utils/styling.ts';
	import { themeStore } from '#lib/shared/stores/theme.svelte.ts';
	import { metaName } from '#lib/i18n/metadata.ts';
	import {
		topology_linkSpeedLegendTitle,
		topology_speedEndsDiffer,
		topology_speedUnknown
	} from '#lib/paraglide/messages.js';

	/**
	 * The key to links coloured by speed: each band's colour and width, a link whose ends differ,
	 * and one whose speed is unknown. Shown only while the view colours links by speed, and kept in
	 * image exports, where the colours would otherwise mean nothing.
	 */
	const shareEdgeColoring = useShareEdgeColoring();
	let shown = $derived(
		effectiveEdgeColoring($topologyOptions.local, $activeView, shareEdgeColoring) === 'Speed'
	);
	// As on the links themselves: an export sets its theme on the document, not in the store.
	let darkTheme = $derived(
		$isExporting
			? typeof document !== 'undefined' && document.documentElement.classList.contains('dark')
			: themeStore.resolvedTheme === 'dark'
	);
	let ramp = $derived(darkTheme ? LINK_SPEED_RAMP.dark : LINK_SPEED_RAMP.light);
	// The two-tone sample uses the slowest and fastest colours, the pair that differs most.
	let splitId = 'link-speed-legend-split';
</script>

{#if shown}
	<Panel position="bottom-right" class="link-speed-legend card">
		<p class="mb-1.5 text-xs font-medium text-secondary">{topology_linkSpeedLegendTitle()}</p>
		<ul class="space-y-1 text-xs text-secondary">
			{#each LINK_SPEED_TIERS as tier, index (tier.id)}
				<li class="flex items-center gap-2">
					<svg width="28" height="8" aria-hidden="true">
						<line
							x1="0"
							y1="4"
							x2="28"
							y2="4"
							stroke={ramp[index]}
							stroke-width={TIER_STROKE_WIDTHS[index]}
						/>
					</svg>
					{metaName('link_speed_tiers', tier.id, tier.name)}
				</li>
			{/each}
			<li class="flex items-center gap-2">
				<svg width="28" height="8" aria-hidden="true">
					<defs>
						<linearGradient id={splitId} x1="0" y1="0" x2="1" y2="0">
							<stop offset="50%" stop-color={ramp[0]} />
							<stop offset="50%" stop-color={ramp[ramp.length - 1]} />
						</linearGradient>
					</defs>
					<rect x="0" y="2.5" width="28" height="3" fill="url(#{splitId})" />
				</svg>
				{topology_speedEndsDiffer()}
			</li>
			<li class="flex items-center gap-2">
				<svg width="28" height="8" aria-hidden="true">
					<line
						x1="0"
						y1="4"
						x2="28"
						y2="4"
						stroke={LINK_SPEED_UNKNOWN}
						stroke-width={UNKNOWN_STROKE_WIDTH}
					/>
				</svg>
				{topology_speedUnknown()}
			</li>
		</ul>
	</Panel>
{/if}

<style>
	/* Above the branding badge in the bottom-right corner. */
	:global(.link-speed-legend) {
		margin-bottom: 3rem;
		padding: 0.5rem 0.75rem;
		pointer-events: none;
	}
</style>

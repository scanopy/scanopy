<script lang="ts">
	import { createColorHelper, type Color } from '#lib/shared/utils/styling.ts';
	import { dependencyColors } from '../../utils';
	import type { Dependency, EdgeStyle } from '../../types/base';
	import { Edit } from '@lucide/svelte';
	import {
		common_bezier,
		common_color,
		common_done,
		common_straight,
		common_style,
		dependencies_edgeColor,
		dependencies_edgeColorHelp,
		dependencies_edgeStyleHelp,
		dependencies_edgeStyleLabel,
		dependencies_editEdgeStyle,
		dependencies_smoothStep
	} from '#lib/paraglide/messages.js';

	let {
		formData = $bindable(),
		collapsed = $bindable(false),
		editable = true,
		showCollapseToggle = true,
		layout = 'vertical',
		onColorChange,
		onEdgeStyleChange
	}: {
		formData: Dependency;
		collapsed?: boolean;
		editable?: boolean;
		showCollapseToggle?: boolean;
		layout?: 'vertical' | 'horizontal';
		onColorChange?: (color: Color) => void;
		onEdgeStyleChange?: (style: EdgeStyle) => void;
	} = $props();

	function handleColorChange(color: Color) {
		if (onColorChange) {
			onColorChange(color);
		} else {
			formData.color = color;
		}
	}

	function handleEdgeStyleChange(style: EdgeStyle) {
		if (onEdgeStyleChange) {
			onEdgeStyleChange(style);
		} else {
			formData.edge_style = style;
		}
	}

	let edgeStyleOptions = $derived([
		{ value: 'Straight' as const, label: common_straight() },
		{ value: 'SmoothStep' as const, label: dependencies_smoothStep() },
		{ value: 'Bezier' as const, label: common_bezier() }
	]);

	// Ensure formData has default values if not set
	$effect(() => {
		if (!formData.color) {
			formData.color = dependencyColors()[0];
		}
		if (!formData.edge_style) {
			formData.edge_style = 'Bezier';
		}
	});

	// A dependency saved before its colour was claimed by another edge type keeps it on offer, so
	// opening the editor shows its current colour selected rather than none.
	const allowedColors = dependencyColors();
	let colorOptions = $derived(
		allowedColors.includes(formData.color as Color)
			? allowedColors
			: [formData.color as Color, ...allowedColors]
	);

	let selectedColorHelper = $derived(createColorHelper(formData.color));
	let selectedEdgeStyleLabel = $derived(
		edgeStyleOptions.find((opt) => opt.value === formData.edge_style)?.label || common_bezier()
	);
</script>

{#if showCollapseToggle && collapsed}
	<!-- Collapsed view -->
	<div class="flex items-center justify-between gap-3">
		<div class="flex items-center gap-3">
			<!-- Color indicator -->
			<div
				class="h-8 w-8 rounded-lg border-2 border-white ring-2 ring-white ring-offset-2"
				style="background-color: {selectedColorHelper.rgb}; --tw-ring-offset-color: var(--color-bg-surface);"
				aria-label="Selected color: {formData.color}"
			></div>

			<!-- Edge style label -->
			<div class="flex flex-col">
				<span class="text-sm font-medium text-primary"
					>{common_style()}: {selectedEdgeStyleLabel}</span
				>
				<span class="text-xs text-tertiary"
					><span class="capitalize">{common_color()}: {formData.color}</span></span
				>
			</div>
		</div>

		<!-- Edit button -->
		<button
			type="button"
			onclick={() => (editable ? (collapsed = false) : {})}
			class="btn-icon"
			disabled={!editable}
			aria-label={dependencies_editEdgeStyle()}
		>
			<Edit size={16} />
		</button>
	</div>
{:else}
	<!-- Expanded view -->
	<div class="space-y-4">
		{#if showCollapseToggle}
			<!-- Header with collapse button -->
			<div class="flex items-center justify-between">
				<div class="block text-sm font-medium text-primary">{dependencies_editEdgeStyle()}</div>
				<button type="button" onclick={() => (collapsed = true)} class="btn-secondary text-xs">
					{common_done()}
				</button>
			</div>
		{/if}

		<div class={layout === 'horizontal' ? 'grid grid-cols-2 gap-6' : 'space-y-6'}>
			<!-- Edge Color Section -->
			<div class="{layout === 'horizontal' ? 'card p-4' : ''} space-y-3">
				<div class="text-sm font-medium text-primary">{dependencies_edgeColor()}</div>
				<p class="text-xs text-tertiary">{dependencies_edgeColorHelp()}</p>

				<div class="grid grid-cols-7 gap-2">
					{#each colorOptions as color (color)}
						{@const colorHelper = createColorHelper(color)}
						<button
							type="button"
							onclick={() => handleColorChange(color)}
							class="group relative aspect-square w-full rounded-lg border-2 transition-all hover:scale-110"
							class:border-gray-300={formData.color !== color}
							class:dark:border-gray-500={formData.color !== color}
							class:border-white={formData.color === color}
							class:ring-2={formData.color === color}
							class:ring-white={formData.color === color}
							class:ring-offset-2={formData.color === color}
							style="background-color: {colorHelper.rgb}; --tw-ring-offset-color: var(--color-bg-surface);"
							aria-label={`Select ${color} color`}
						>
							{#if formData.color === color}
								<div class="absolute inset-0 flex items-center justify-center">
									<svg
										class="h-4 w-4 text-white drop-shadow-lg"
										fill="currentColor"
										viewBox="0 0 20 20"
									>
										<path
											fill-rule="evenodd"
											d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
											clip-rule="evenodd"
										/>
									</svg>
								</div>
							{/if}
						</button>
					{/each}
				</div>
			</div>

			<!-- Edge Style Section -->
			<div class="{layout === 'horizontal' ? 'card p-4' : ''} space-y-3">
				<div class="text-sm font-medium text-primary">{dependencies_edgeStyleLabel()}</div>
				<p class="text-xs text-tertiary">{dependencies_edgeStyleHelp()}</p>

				<div class="space-y-1">
					{#each edgeStyleOptions as option (option.value)}
						<button
							type="button"
							onclick={() => handleEdgeStyleChange(option.value)}
							class="flex w-full items-center gap-2 rounded-md border px-3 py-1.5 text-left transition-all"
							class:border-gray-300={formData.edge_style !== option.value}
							class:dark:border-gray-600={formData.edge_style !== option.value}
							class:border-blue-500={formData.edge_style === option.value}
							class:bg-blue-900-20={formData.edge_style === option.value}
							class:ring-1={formData.edge_style === option.value}
							class:ring-blue-500={formData.edge_style === option.value}
						>
							<div
								class="flex h-4 w-4 items-center justify-center rounded-full border-2 transition-all"
								class:border-gray-300={formData.edge_style !== option.value}
								class:dark:border-gray-500={formData.edge_style !== option.value}
								class:border-blue-500={formData.edge_style === option.value}
								class:bg-blue-500={formData.edge_style === option.value}
							>
								{#if formData.edge_style === option.value}
									<div class="h-1.5 w-1.5 rounded-full bg-white"></div>
								{/if}
							</div>
							<span
								class="text-sm font-medium transition-colors"
								class:text-secondary={formData.edge_style !== option.value}
								class:text-blue-600={formData.edge_style === option.value}
								class:dark:text-blue-400={formData.edge_style === option.value}
							>
								{option.label}
							</span>
						</button>
					{/each}
				</div>
			</div>
		</div>
	</div>
{/if}

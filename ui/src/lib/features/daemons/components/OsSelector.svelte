<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { DaemonOS } from '../utils';
	import OsIcon from './OsIcon.svelte';
	import {
		common_binary,
		common_docker,
		common_exe,
		common_linux,
		common_macos,
		common_msi,
		common_windows,
		daemons_operatingSystem
	} from '$lib/paraglide/messages';

	type LinuxMethod = 'binary' | 'docker';
	type WindowsMethod = 'exe' | 'msi';

	interface Props {
		selectedOS: DaemonOS;
		onOsSelect: (os: DaemonOS) => void;
		linuxMethod?: LinuxMethod;
		onLinuxMethodChange?: (method: LinuxMethod) => void;
		windowsMethod?: WindowsMethod;
		onWindowsMethodChange?: (method: WindowsMethod) => void;
		afterLabel?: Snippet;
		afterButtons?: Snippet;
		children?: Snippet;
		/** Show the OS buttons. The create-daemon flow picks the OS in its first step and shows only
		 *  the install-method toggles at install time. */
		showOs?: boolean;
		/** Show the Linux and Windows install-method toggles. */
		showMethods?: boolean;
		/** Grey out the OS buttons, e.g. once credentials were created for the chosen OS. */
		osDisabled?: boolean;
	}

	let {
		selectedOS,
		onOsSelect,
		linuxMethod = 'binary',
		onLinuxMethodChange,
		windowsMethod = 'exe',
		onWindowsMethodChange,
		afterLabel,
		afterButtons,
		children,
		showOs = true,
		showMethods = true,
		osDisabled = false
	}: Props = $props();

	let osOptions = $derived([
		{ id: 'linux' as DaemonOS, label: common_linux() },
		{ id: 'macos' as DaemonOS, label: common_macos() },
		{ id: 'windows' as DaemonOS, label: common_windows() },
		{ id: 'freebsd' as DaemonOS, label: 'FreeBSD' }
	]);

	let linuxMethodOptions = $derived([
		{ id: 'binary' as LinuxMethod, label: common_binary() },
		{ id: 'docker' as LinuxMethod, label: common_docker() }
	]);

	let windowsMethodOptions = $derived([
		{ id: 'exe' as WindowsMethod, label: common_exe() },
		{ id: 'msi' as WindowsMethod, label: common_msi() }
	]);
</script>

{#if showOs}
	<!-- OS Selector: Desktop layout -->
	<div role="group" aria-label={daemons_operatingSystem()} class="hidden sm:block">
		<div class="flex items-baseline justify-between">
			<span class="text-secondary text-sm font-medium">{daemons_operatingSystem()}</span>
			{@render afterLabel?.()}
		</div>
		<div class="mt-2 flex items-center justify-between gap-2">
			<div class="flex items-center gap-2">
				{#each osOptions as option (option.id)}
					<button
						type="button"
						class="btn-secondary flex items-center gap-1.5 {selectedOS === option.id
							? 'ring-primary ring-2'
							: ''}"
						disabled={osDisabled}
						onclick={() => onOsSelect(option.id)}
					>
						<OsIcon os={option.id} class="h-4 w-4 flex-shrink-0" />
						{option.label}
					</button>
				{/each}
			</div>
			{@render afterButtons?.()}
		</div>
	</div>

	<!-- OS Selector: Mobile layout -->
	<div role="group" aria-label={daemons_operatingSystem()} class="sm:hidden">
		<span class="text-secondary mb-1 block text-sm font-medium">{daemons_operatingSystem()}</span>
		<div class="flex items-stretch gap-2">
			<select
				class="input-field flex-1"
				value={selectedOS}
				disabled={osDisabled}
				onchange={(e) => onOsSelect(e.currentTarget.value as DaemonOS)}
			>
				{#each osOptions as option (option.id)}
					<option value={option.id}>{option.label}</option>
				{/each}
			</select>
			{@render afterButtons?.()}
		</div>
		{#if afterLabel}
			<div class="mt-1">
				{@render afterLabel()}
			</div>
		{/if}
	</div>
{:else if afterLabel || afterButtons}
	<!-- OS chosen elsewhere: keep the caller's hint and buttons on their own row. -->
	<div class="flex items-center justify-between gap-2">
		<div>{@render afterLabel?.()}</div>
		{@render afterButtons?.()}
	</div>
{/if}

{#if showMethods && selectedOS === 'linux'}
	<!-- Linux: Install method sub-toggle (binary vs docker) -->
	<div class="flex gap-1 sm:w-[calc((100%-3*0.5rem)/4)]">
		{#each linuxMethodOptions as option (option.id)}
			<button
				type="button"
				class="btn-secondary btn-sm flex-1 {linuxMethod === option.id ? 'ring-primary ring-2' : ''}"
				onclick={() => onLinuxMethodChange?.(option.id)}
			>
				{option.label}
			</button>
		{/each}
	</div>
{:else if showMethods && selectedOS === 'windows' && onWindowsMethodChange}
	<!-- Windows: Install method sub-toggle (exe vs msi) — only shown when the caller wires it up,
	     since not every OsSelector consumer has distinct content for each method. -->
	<div class="flex gap-1 sm:w-[calc((100%-3*0.5rem)/4)]">
		{#each windowsMethodOptions as option (option.id)}
			<button
				type="button"
				class="btn-secondary btn-sm flex-1 {windowsMethod === option.id
					? 'ring-primary ring-2'
					: ''}"
				onclick={() => onWindowsMethodChange?.(option.id)}
			>
				{option.label}
			</button>
		{/each}
	</div>
{/if}

{@render children?.()}

<script lang="ts">
	/**
	 * Daemon management modal: edit the server-side record, and manage the daemon's 1:1 API key.
	 *
	 * Deliberately SEPARATE from CreateDaemonModal, counter to the usual shared create/edit
	 * modal pattern — installing a daemon and administering an installed one are different
	 * jobs with different affordances. Do not unify them.
	 */
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { UserDisplay } from '$lib/shared/components/forms/selection/display/UserDisplay.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { Info, KeyRound } from 'lucide-svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { ApiKey } from '$lib/features/daemon_api_keys/types/base';
	import {
		useUpdateDaemonMutation,
		useDaemonInstallCommandQuery
	} from '$lib/features/daemons/queries';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import OsSelector from './OsSelector.svelte';
	import {
		useApiKeysQuery,
		useUpdateApiKeyMutation,
		useRotateApiKeyMutation
	} from '$lib/features/daemon_api_keys/queries';
	import { useUsersQuery } from '$lib/features/users/queries';
	import ApiKeyFormFields from '$lib/features/daemon_api_keys/components/ApiKeyFormFields.svelte';
	import { createApiKeyForm } from '$lib/features/daemon_api_keys/form';
	import DaemonKeyAssociation from './DaemonKeyAssociation.svelte';
	import { constructDaemonUrl, type DaemonOS } from '$lib/features/daemons/utils';
	import { osInstallCommand } from '$lib/features/daemons/types/base';
	import {
		common_apiKey,
		common_cancel,
		common_close,
		common_details,
		common_editName,
		common_name,
		common_port,
		common_save,
		common_saving,
		common_tags,
		daemons_config_daemonUrl,
		daemons_config_daemonUrlHelpNoPort,
		daemons_config_mode,
		daemons_config_portHelpServerPoll,
		daemons_readOnlyFieldHelp,
		daemons_reconfigureDockerHint,
		daemons_reconfigureSectionHelp,
		daemons_reconfigureSectionTitle,
		common_maintainer,
		common_maintainerHelp
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		daemon: Daemon | null;
		name?: string;
	}

	let { isOpen = false, onClose, daemon = null, name = undefined }: Props = $props();

	const updateDaemonMutation = useUpdateDaemonMutation();
	const updateApiKeyMutation = useUpdateApiKeyMutation();
	const rotateApiKeyMutation = useRotateApiKeyMutation();
	const apiKeysQuery = useApiKeysQuery();
	const usersQuery = useUsersQuery();

	let activeTab = $state('details');
	let loading = $state(false);
	let generatedKey = $state<string | null>(null);

	let tabs: ModalTab[] = $derived([
		{ id: 'details', label: common_details(), icon: Info },
		{ id: 'apiKey', label: common_apiKey(), icon: KeyRound }
	]);

	let isServerPoll = $derived(daemon?.mode === 'server_poll');
	// The daemon record already carries api_key_id, so the bound key resolves from the list
	// the keys tab already loads — no per-daemon endpoint needed.
	let daemonKey = $derived<ApiKey | null>(
		daemon?.api_key_id
			? ((apiKeysQuery.data ?? []).find((k) => k.id === daemon.api_key_id) ?? null)
			: null
	);

	let usersData = $derived(usersQuery.data ?? []);

	// Reconfigure command for the Details tab. Only fetched while the modal is open, and only
	// meaningful for an installed daemon — one that has never checked in is still following its
	// install command.
	const installCommandQuery = useDaemonInstallCommandQuery(
		() => daemon?.id ?? null,
		() => ({ purpose: 'reconfigure' }),
		{ enabled: () => isOpen && daemon?.last_seen != null }
	);
	let syncOs = $state<DaemonOS>('linux');
	let syncLinuxMethod = $state<'binary' | 'docker'>('binary');
	let syncIsDocker = $derived(syncOs === 'linux' && syncLinuxMethod === 'docker');
	let hasReconfigure = $derived(installCommandQuery.data != null);
	// Binary reconfigure is a command; docker is the env vars the operator swaps into their
	// own compose (a reconfigure doesn't hand back a whole replacement compose).
	let syncCommand = $derived(
		installCommandQuery.data && !syncIsDocker
			? osInstallCommand(installCommandQuery.data, syncOs)
			: ''
	);
	let syncDockerEnv = $derived(installCommandQuery.data?.docker.env ?? []);
	let syncLanguage = $derived(syncOs === 'windows' ? 'powershell' : 'bash');

	const DEFAULT_DAEMON_PORT = 60073;

	// ServerPoll stores one url; the form edits host and port separately (as the create modal
	// does) and recombines them on submit.
	function splitUrl(url: string): { base: string; port: number } {
		try {
			const parsed = new globalThis.URL(url);
			const port = parsed.port ? Number(parsed.port) : parsed.protocol === 'https:' ? 443 : 80;
			const path = parsed.pathname === '/' ? '' : parsed.pathname;
			return { base: `${parsed.protocol}//${parsed.hostname}${path}`, port };
		} catch {
			return { base: url, port: DEFAULT_DAEMON_PORT };
		}
	}

	const detailsForm = createForm(() => ({
		defaultValues: {
			name: '',
			url: '',
			port: DEFAULT_DAEMON_PORT,
			user_id: '',
			tags: [] as string[]
		},
		onSubmit: async ({ value }) => {
			if (!daemon) return;
			loading = true;
			try {
				await updateDaemonMutation.mutateAsync({
					...daemon,
					name: value.name,
					user_id: value.user_id,
					tags: value.tags,
					// Only ServerPoll has a meaningful url; the server rejects changing it otherwise.
					url: isServerPoll ? constructDaemonUrl(value.url, Number(value.port)) : daemon.url
				});
				onClose();
			} finally {
				loading = false;
			}
		}
	}));

	const keyForm = createApiKeyForm(async (value) => {
		loading = true;
		try {
			await updateApiKeyMutation.mutateAsync(value);
			onClose();
		} finally {
			loading = false;
		}
	});

	// Which key the key form currently holds. TanStack form values aren't tracked by
	// `$derived`, so the effect below can't read them back to decide whether to seed.
	let seededKeyId = $state<string | null>(null);

	function handleOpen() {
		activeTab = 'details';
		generatedKey = null;
		seededKeyId = null;
		if (!daemon) return;

		const { base, port } = splitUrl(daemon.url);
		detailsForm.reset({
			name: daemon.name,
			url: isServerPoll ? base : '',
			port,
			user_id: daemon.user_id,
			tags: daemon.tags ?? []
		});
	}

	// The key list is fetched separately and can arrive after the modal opens, so seed the
	// key form whenever the resolved key changes rather than only on open.
	$effect(() => {
		if (isOpen && daemonKey && daemonKey.id !== seededKeyId) {
			seededKeyId = daemonKey.id;
			keyForm.reset({ ...daemonKey });
		}
	});

	async function handleRotateKey() {
		if (!daemonKey) return;
		loading = true;
		try {
			generatedKey = await rotateApiKeyMutation.mutateAsync(daemonKey.id);
		} catch {
			// The API client reports the failure.
		} finally {
			loading = false;
		}
	}

	async function handleSubmit() {
		await submitForm(activeTab === 'details' ? detailsForm : keyForm);
	}

	// Saving only applies to a tab with an editable form — the association CTA has its own action.
	let canSave = $derived(activeTab === 'details' || daemonKey != null);

	let colorHelper = $derived(entities.getColorHelper('Daemon'));
</script>

<GenericModal
	{isOpen}
	{name}
	title={common_editName({ name: daemon?.name ?? '' })}
	entityId={daemon?.id}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
	{tabs}
	{activeTab}
	onTabChange={(id) => (activeTab = id)}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Daemon')} color={colorHelper.color} />
	{/snippet}

	{#if daemon}
		<form
			onsubmit={(e) => {
				e.preventDefault();
				e.stopPropagation();
				handleSubmit();
			}}
			class="flex min-h-0 flex-1 flex-col"
		>
			<div class="min-h-0 flex-1 overflow-auto p-6">
				<!-- Details -->
				<div class="space-y-4" class:hidden={activeTab !== 'details'}>
					<!-- Name and mode are both read-only, so they share a row. Name is the
					     server-authoritative label (reaches the daemon via its handshake, not a
					     command); mode is fixed at install and decides how daemon and server reach
					     each other. Neither is a form value the user can change here. -->
					<div class="grid grid-cols-2 gap-4">
						<detailsForm.Field name="name">
							{#snippet children(field)}
								<TextInput
									label={common_name()}
									id="daemon-name"
									{field}
									disabled
									helpText={daemons_readOnlyFieldHelp()}
								/>
							{/snippet}
						</detailsForm.Field>

						<div class="space-y-2">
							<label for="daemon-mode" class="text-secondary block text-sm font-medium">
								{daemons_config_mode()}
							</label>
							<select id="daemon-mode" class="input-field" disabled value={daemon.mode}>
								<option value="daemon_poll">daemon_poll</option>
								<option value="server_poll">server_poll</option>
							</select>
							<p class="text-tertiary text-xs">{daemons_readOnlyFieldHelp()}</p>
						</div>
					</div>

					{#if isServerPoll}
						<div class="grid grid-cols-[1fr_auto] gap-4">
							<detailsForm.Field name="url" validators={{ onBlur: ({ value }) => required(value) }}>
								{#snippet children(field)}
									<TextInput
										label={daemons_config_daemonUrl()}
										id="daemon-url"
										{field}
										helpText={daemons_config_daemonUrlHelpNoPort()}
										required
									/>
								{/snippet}
							</detailsForm.Field>

							<div class="w-48">
								<detailsForm.Field name="port">
									{#snippet children(field)}
										<TextInput
											label={common_port()}
											id="daemon-port"
											type="number"
											{field}
											helpText={daemons_config_portHelpServerPoll()}
										/>
									{/snippet}
								</detailsForm.Field>
							</div>
						</div>
					{/if}

					<detailsForm.Field name="user_id">
						{#snippet children(field)}
							<RichSelect
								label={common_maintainer()}
								selectedValue={field.state.value}
								options={usersData}
								onSelect={(value) => field.handleChange(value)}
								displayComponent={UserDisplay}
								showSearch={true}
								helpText={common_maintainerHelp()}
							/>
						{/snippet}
					</detailsForm.Field>

					<detailsForm.Field name="tags">
						{#snippet children(field)}
							<TagPicker
								label={common_tags()}
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</detailsForm.Field>

					<!-- Server-held config the daemon may have drifted from — most usefully the
					     ServerPoll port, which the server dials but the daemon must bind. The
					     command carries no credential, so it is safe to show here and to run
					     repeatedly. -->
					{#if hasReconfigure}
						<div class="card space-y-3">
							<p class="text-primary text-sm">{daemons_reconfigureSectionTitle()}</p>
							<p class="text-tertiary text-sm">{daemons_reconfigureSectionHelp()}</p>
							<OsSelector
								selectedOS={syncOs}
								onOsSelect={(os) => (syncOs = os)}
								linuxMethod={syncLinuxMethod}
								onLinuxMethodChange={(method) => (syncLinuxMethod = method)}
							>
								{#if syncIsDocker}
									<p class="text-secondary text-sm">{daemons_reconfigureDockerHint()}</p>
									<CodeContainer
										language="yaml"
										expandable={false}
										maxHeight=""
										code={syncDockerEnv.join('\n')}
										preventSelect={true}
									/>
								{:else}
									<CodeContainer
										language={syncLanguage}
										expandable={false}
										maxHeight=""
										code={syncCommand}
										preventSelect={true}
									/>
								{/if}
							</OsSelector>
						</div>
					{/if}
				</div>

				<!-- API key -->
				<div class:hidden={activeTab !== 'apiKey'}>
					{#if daemonKey}
						<!-- The key belongs to this daemon, so its name/tags aren't user-managed
						     here, and the tab already labels the section. -->
						<ApiKeyFormFields
							form={keyForm}
							isEditing={true}
							{generatedKey}
							{loading}
							onGenerate={() => {}}
							onRotate={handleRotateKey}
							showNetwork={false}
							showName={false}
							showTags={false}
							showHeading={false}
						/>
					{:else}
						<DaemonKeyAssociation {daemon} />
					{/if}
				</div>
			</div>

			<EntityMetadataSection entities={[daemon]} />

			<div class="modal-footer">
				<div class="flex items-center justify-end gap-3">
					<button type="button" disabled={loading} onclick={onClose} class="btn-secondary">
						{canSave ? common_cancel() : common_close()}
					</button>
					{#if canSave}
						<button type="submit" disabled={loading} class="btn-primary">
							{loading ? common_saving() : common_save()}
						</button>
					{/if}
				</div>
			</div>
		</form>
	{/if}
</GenericModal>

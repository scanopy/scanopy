import {
	portRangeValidation,
	required,
	max,
	ipAddressFormat,
	min,
	urlWithoutPort,
	type Validator
} from '$lib/shared/components/forms/validators';
import * as m from '$lib/paraglide/messages';

interface FieldDef {
	id: string;
	label: () => string;
	type: 'string' | 'number' | 'boolean' | 'select';
	defaultValue?: string | number | boolean;
	cliFlag: string;
	envVar: string;
	helpText: () => string;
	section?: () => string; // undefined = basic field, string = advanced section name
	placeholder?: string | number | (() => string | number);
	options?: { label: () => string; value: string }[];
	disabled?: (isNew: boolean) => boolean;
	validators?: Validator[];
	required?: boolean;
	showWhen?: (values: Record<string, string | number | boolean>) => boolean;
	docsOnly?: boolean;
}

export const fieldDefs: FieldDef[] = [
	{
		id: 'serverUrl',
		label: () => m.daemons_config_serverUrl(),
		type: 'string',
		cliFlag: '--server-url',
		envVar: 'SCANOPY_SERVER_URL',
		helpText: () => m.daemons_config_serverUrlHelp(),
		defaultValue: 'http://127.0.0.1:60072',
		docsOnly: true
	},
	{
		id: 'daemonApiKey',
		label: () => m.common_apiKey(),
		type: 'string',
		cliFlag: '--daemon-api-key',
		envVar: 'SCANOPY_DAEMON_API_KEY',
		helpText: () => m.daemons_config_apiKeyHelp(),
		required: true,
		docsOnly: true
	},
	{
		id: 'networkId',
		label: () => m.daemons_config_networkId(),
		type: 'string',
		cliFlag: '--network-id',
		envVar: 'SCANOPY_NETWORK_ID',
		helpText: () => m.daemons_config_networkIdHelp(),
		docsOnly: true
	},
	// UI form fields
	{
		id: 'name',
		label: () => m.common_name(),
		type: 'string',
		cliFlag: '--name',
		envVar: 'SCANOPY_NAME',
		helpText: () => m.daemons_config_nameHelp(),
		placeholder: () => m.daemons_config_namePlaceholder(),
		defaultValue: 'scanopy-daemon',
		validators: [required, max(100)],
		required: true
	},
	{
		id: 'mode',
		label: () => m.daemons_config_mode(),
		type: 'select',
		defaultValue: 'daemon_poll',
		cliFlag: '--mode',
		envVar: 'SCANOPY_MODE',
		helpText: () => m.daemons_config_modeHelp(),
		options: [
			{ label: () => m.daemons_mode_daemonPoll(), value: 'daemon_poll' },
			{ label: () => m.daemons_mode_serverPoll(), value: 'server_poll' }
		],
		disabled: (isNew) => !isNew
	},
	{
		id: 'daemonUrl',
		label: () => m.daemons_config_daemonUrl(),
		type: 'string',
		defaultValue: '',
		cliFlag: '--daemon-url',
		envVar: 'SCANOPY_DAEMON_URL',
		helpText: () => m.daemons_config_daemonUrlHelpNoPort(),
		placeholder: () => m.common_placeholderDaemonUrlNoPort(),
		validators: [required, urlWithoutPort],
		required: true,
		showWhen: (values) => values.mode === 'server_poll'
	},
	{
		id: 'daemonPort',
		label: () => m.common_port(),
		type: 'number',
		placeholder: 60073,
		cliFlag: '--daemon-port',
		envVar: 'SCANOPY_DAEMON_PORT',
		helpText: () => m.daemons_config_portHelpServerPoll(),
		validators: [portRangeValidation],
		showWhen: (values) => values.mode === 'server_poll'
	},
	// Runtime
	{
		id: 'interfaces',
		label: () => m.common_interfaces(),
		type: 'string',
		defaultValue: '',
		cliFlag: '--interfaces',
		envVar: 'SCANOPY_INTERFACES',
		helpText: () => m.daemons_config_interfacesHelp(),
		placeholder: () => m.common_placeholderInterface(),
		section: () => m.common_runtime()
	},
	// Server Connection
	{
		id: 'bindAddress',
		label: () => m.daemons_config_bindAddress(),
		type: 'string',
		defaultValue: '',
		cliFlag: '--bind-address',
		envVar: 'SCANOPY_BIND_ADDRESS',
		helpText: () => m.daemons_config_bindAddressHelp(),
		placeholder: '0.0.0.0',
		section: () => m.daemons_config_sectionServerConnection(),
		validators: [ipAddressFormat],
		showWhen: (values) => values.mode === 'server_poll'
	},
	{
		id: 'allowSelfSignedCerts',
		label: () => m.daemons_config_allowSelfSignedCerts(),
		type: 'boolean',
		defaultValue: false,
		cliFlag: '--allow-self-signed-certs',
		envVar: 'SCANOPY_ALLOW_SELF_SIGNED_CERTS',
		helpText: () => m.daemons_config_allowSelfSignedCertsHelp(),
		section: () => m.daemons_config_sectionServerConnection(),
		showWhen: (values) => values.mode === 'daemon_poll'
	},
	// Runtime
	{
		id: 'logLevel',
		label: () => m.daemons_config_logLevel(),
		type: 'select',
		defaultValue: 'info',
		cliFlag: '--log-level',
		envVar: 'SCANOPY_LOG_LEVEL',
		helpText: () => m.daemons_config_logLevelHelp(),
		section: () => m.common_runtime(),
		options: [
			{ label: () => m.common_trace(), value: 'trace' },
			{ label: () => m.common_debug(), value: 'debug' },
			{ label: () => m.common_info(), value: 'info' },
			{ label: () => m.common_warn(), value: 'warn' },
			{ label: () => m.common_error(), value: 'error' }
		]
	},
	{
		id: 'heartbeatInterval',
		label: () => m.daemons_config_heartbeatInterval(),
		type: 'number',
		placeholder: 30,
		cliFlag: '--heartbeat-interval',
		envVar: 'SCANOPY_HEARTBEAT_INTERVAL',
		helpText: () => m.daemons_config_heartbeatIntervalHelp(),
		section: () => m.common_runtime(),
		validators: [min(0), max(300)],
		showWhen: (values) => values.mode === 'daemon_poll'
	}
];

export function getVisibleFieldIds(values: Record<string, string | number | boolean>): Set<string> {
	return new Set(fieldDefs.filter((d) => !d.showWhen || d.showWhen(values)).map((d) => d.id));
}

export const sectionDefs: Record<
	string,
	{
		description: () => string;
		docsHint?: { text: () => string; href: string; linkText: () => string };
	}
> = {
	'Server Connection': { description: () => m.daemons_config_sectionServerConnectionDesc() },
	Runtime: { description: () => m.daemons_config_sectionRuntimeDesc() }
};

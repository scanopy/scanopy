import {
	portRangeValidation,
	required,
	max,
	ipAddressFormat,
	min,
	url,
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
		validators: [required, max(100)],
		required: true
	},
	{
		id: 'mode',
		label: () => m.daemons_config_mode(),
		type: 'select',
		defaultValue: 'server_poll',
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
	// Docker Discovery
	{
		id: 'dockerProxy',
		label: () => m.daemons_config_dockerProxy(),
		type: 'string',
		defaultValue: '',
		cliFlag: '--docker-proxy',
		envVar: 'SCANOPY_DOCKER_PROXY',
		helpText: () => m.daemons_config_dockerProxyHelp(),
		placeholder: () => m.common_placeholderLocalHostName(),
		section: () => m.daemons_config_sectionDockerDiscovery(),
		validators: [url]
	},
	{
		id: 'dockerProxySslCert',
		label: () => m.daemons_config_dockerProxySslCert(),
		type: 'string',
		defaultValue: '',
		cliFlag: '--docker-proxy-ssl-cert',
		envVar: 'SCANOPY_DOCKER_PROXY_SSL_CERT',
		helpText: () => m.daemons_config_dockerProxySslCertHelp(),
		placeholder: () => m.common_placeholderSslCert(),
		section: () => m.daemons_config_sectionDockerDiscovery(),
		validators: []
	},
	{
		id: 'dockerProxySslKey',
		label: () => m.daemons_config_dockerProxySslKey(),
		type: 'string',
		defaultValue: '',
		cliFlag: '--docker-proxy-ssl-key',
		envVar: 'SCANOPY_DOCKER_PROXY_SSL_KEY',
		helpText: () => m.daemons_config_dockerProxySslKeyHelp(),
		placeholder: () => m.common_placeholderSslKey(),
		section: () => m.daemons_config_sectionDockerDiscovery(),
		validators: []
	},
	{
		id: 'dockerProxySslChain',
		label: () => m.daemons_config_dockerProxySslChain(),
		type: 'string',
		defaultValue: '',
		cliFlag: '--docker-proxy-ssl-chain',
		envVar: 'SCANOPY_DOCKER_PROXY_SSL_CHAIN',
		helpText: () => m.daemons_config_dockerProxySslChainHelp(),
		placeholder: () => m.common_placeholderSslChain(),
		section: () => m.daemons_config_sectionDockerDiscovery(),
		validators: []
	},
	// Network Discovery
	{
		id: 'interfaces',
		label: () => m.common_interfaces(),
		type: 'string',
		defaultValue: '',
		cliFlag: '--interfaces',
		envVar: 'SCANOPY_INTERFACES',
		helpText: () => m.daemons_config_interfacesHelp(),
		placeholder: () => m.common_placeholderInterface(),
		section: () => m.daemons_config_sectionNetworkDiscovery()
	},
	{
		id: 'arp_retries',
		label: () => m.daemons_config_arpRetries(),
		type: 'number',
		cliFlag: '--arp-retries',
		envVar: 'SCANOPY_ARP_RETRIES',
		placeholder: 2,
		helpText: () => m.daemons_config_arpRetriesHelp(),
		section: () => m.daemons_config_sectionNetworkDiscovery()
	},
	{
		id: 'scan_rate_pps',
		label: () => m.daemons_config_portScanPacketsPerSecond(),
		type: 'number',
		cliFlag: '--scan-rate-pps',
		envVar: 'SCANOPY_SCAN_RATE_PPS',
		placeholder: 500,
		helpText: () => m.daemons_config_portScanPacketsPerSecondHelp(),
		section: () => m.daemons_config_sectionNetworkDiscovery()
	},
	{
		id: 'port_scan_batch_size',
		label: () => m.daemons_config_portScanBatchSize(),
		type: 'number',
		cliFlag: '--port-scan-batch-size',
		envVar: 'SCANOPY_PORT_SCAN_BATCH_SIZE',
		placeholder: 200,
		helpText: () => m.daemons_config_portScanBatchSizeHelp(),
		section: () => m.daemons_config_sectionNetworkDiscovery(),
		validators: [min(16), max(1000)]
	},
	{
		id: 'arp_rate_pps',
		label: () => m.daemons_config_arpPacketsPerSecond(),
		type: 'number',
		placeholder: 50,
		cliFlag: '--arp-rate-pps',
		envVar: 'SCANOPY_ARP_RATE_PPS',
		helpText: () => m.daemons_config_arpPacketsPerSecondHelp(),
		section: () => m.daemons_config_sectionNetworkDiscovery()
	},
	{
		id: 'use_npcap_arp',
		label: () => m.daemons_config_useNpcapArp(),
		type: 'boolean',
		defaultValue: false,
		cliFlag: '--use-npcap-arp',
		envVar: 'SCANOPY_USE_NPCAP_ARP',
		helpText: () => m.daemons_config_useNpcapArpHelp(),
		section: () => m.daemons_config_sectionNetworkDiscovery()
	},
	{
		id: 'acceptInvalidScanCerts',
		label: () => m.daemons_config_acceptInvalidScanCerts(),
		type: 'boolean',
		defaultValue: true,
		cliFlag: '--accept-invalid-scan-certs',
		envVar: 'SCANOPY_ACCEPT_INVALID_SCAN_CERTS',
		helpText: () => m.daemons_config_acceptInvalidScanCertsHelp(),
		section: () => m.daemons_config_sectionNetworkDiscovery()
	},
	{
		id: 'concurrentScans',
		label: () => m.daemons_config_concurrentScans(),
		type: 'number',
		cliFlag: '--concurrent-scans',
		envVar: 'SCANOPY_CONCURRENT_SCANS',
		helpText: () => m.daemons_config_concurrentScansHelp(),
		placeholder: () => m.common_auto(),
		section: () => m.daemons_config_sectionNetworkDiscovery()
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
	// Performance
	{
		id: 'logLevel',
		label: () => m.daemons_config_logLevel(),
		type: 'select',
		defaultValue: 'info',
		cliFlag: '--log-level',
		envVar: 'SCANOPY_LOG_LEVEL',
		helpText: () => m.daemons_config_logLevelHelp(),
		section: () => m.common_performance(),
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
		section: () => m.common_performance(),
		validators: [min(0), max(300)],
		showWhen: (values) => values.mode === 'daemon_poll'
	}
];

export const sectionDefs: Record<string, { description: () => string; docsHint?: () => string }> = {
	'Docker Discovery': {
		description: () => m.daemons_config_sectionDockerDiscoveryDesc(),
		docsHint: () => m.daemons_docsDockerProxy()
	},
	'Network Discovery': { description: () => m.daemons_config_sectionNetworkDiscoveryDesc() },
	'Server Connection': { description: () => m.daemons_config_sectionServerConnectionDesc() },
	Performance: { description: () => m.daemons_config_sectionPerformanceDesc() }
};

<script lang="ts">
	import { entities } from '$lib/shared/stores/metadata';
	import type { components } from '$lib/api/schema';

	type NetworkSummary = components['schemas']['NetworkSummary'];

	let { network }: { network: NetworkSummary } = $props();

	const HostIcon = entities.getIconComponent('Host');
	const ServiceIcon = entities.getIconComponent('Service');
	const SubnetIcon = entities.getIconComponent('Subnet');
	const DaemonIcon = entities.getIconComponent('Daemon');

	const hostColor = entities.getColorHelper('Host').icon;
	const serviceColor = entities.getColorHelper('Service').icon;
	const subnetColor = entities.getColorHelper('Subnet').icon;
	const daemonColor = entities.getColorHelper('Daemon').icon;
</script>

<div class="card card-static">
	<h4 class="text-primary mb-3 truncate text-sm font-semibold">{network.name}</h4>
	<div class="grid grid-cols-2 gap-3">
		<div class="flex items-center gap-2">
			<HostIcon class="h-4 w-4 flex-shrink-0" style="color: {hostColor}" />
			<span class="text-secondary text-sm">{network.host_count} hosts</span>
		</div>
		<div class="flex items-center gap-2">
			<ServiceIcon class="h-4 w-4 flex-shrink-0" style="color: {serviceColor}" />
			<span class="text-secondary text-sm">{network.service_count} services</span>
		</div>
		<div class="flex items-center gap-2">
			<SubnetIcon class="h-4 w-4 flex-shrink-0" style="color: {subnetColor}" />
			<span class="text-secondary text-sm">{network.subnet_count} subnets</span>
		</div>
		<div class="flex items-center gap-2">
			<DaemonIcon class="h-4 w-4 flex-shrink-0" style="color: {daemonColor}" />
			<span class="text-secondary text-sm">{network.daemon_count} daemons</span>
		</div>
	</div>
</div>

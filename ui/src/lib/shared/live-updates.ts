import { queryClient, queryKeys } from '$lib/api/query-client';
import type { components } from '$lib/api/schema';
import { discoveryStream } from '$lib/features/discovery/queries';
import { invalidateAllLiveTopology, topologyStream } from '$lib/features/topology/queries';
import { BaseSSEManager, type SSEConfig } from '$lib/shared/utils/sse';

type LiveUpdate = components['schemas']['LiveUpdate'];

/**
 * The app's one live-update stream: discovery progress, daemon check-ins and topology changes. The server sends
 * topology events only when the plan includes the topology view, reading the plan at connect.
 */
class LiveUpdatesSSEManager extends BaseSSEManager<LiveUpdate> {
	protected createConfig(): SSEConfig<LiveUpdate> {
		return {
			url: '/api/v1/live-updates/stream',
			onMessage: async (update) => {
				switch (update.type) {
					case 'Discovery':
						await discoveryStream.handleUpdate(update.data);
						// A finished run can record the first scan and raise or clear Setup items.
						if (update.data.phase === 'Complete') {
							queryClient.invalidateQueries({ queryKey: queryKeys.dashboard.all });
							queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
						}
						break;
					case 'Topology':
						topologyStream.handleUpdate(update.data);
						break;
					// A daemon checked in on a new version or health: its row, its upgrade modal and Home.
					case 'Daemon':
						queryClient.invalidateQueries({ queryKey: queryKeys.daemons.all });
						queryClient.invalidateQueries({ queryKey: queryKeys.dashboard.all });
						queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
						break;
				}
			},
			onError: () => discoveryStream.lost(),
			onOpen: () => discoveryStream.opened()
		};
	}

	/** Updates sent while the stream was closed were missed, so refetch what they refresh. */
	protected override onReconnected() {
		for (const queryKey of [
			queryKeys.discovery.all,
			queryKeys.hosts.all,
			queryKeys.services.all,
			queryKeys.subnets.all,
			queryKeys.daemons.all,
			queryKeys.dashboard.all,
			queryKeys.organizations.current()
		]) {
			queryClient.invalidateQueries({ queryKey });
		}
		invalidateAllLiveTopology();
	}

	protected override onPaused() {
		discoveryStream.reset();
		topologyStream.reset();
	}

	override disconnect() {
		super.disconnect();
		this.onPaused();
	}
}

export const liveUpdatesSSEManager = new LiveUpdatesSSEManager();

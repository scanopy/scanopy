import { useServicesQuery } from '$lib/features/services/queries';
import type { Service } from '$lib/features/services/types/base';
import type { HostDisplayContext } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
import { useHostPickerQuery } from './queries';
import type { HostWithAddresses, IPAddress } from './types/base';

/**
 * The services every host row and card shows as its icon and tags: the hosts' own services,
 * without the unclaimed-open-ports placeholder, which names no service.
 *
 * Fetched for the given hosts only, so a picker pays for the rows it has loaded rather than for
 * every service on the network.
 */
export function useHostServices(getHostIds: () => string[]) {
	const query = useServicesQuery(() => {
		const host_ids = getHostIds();
		return {
			host_ids,
			limit: 0,
			exclude_categories: ['OpenPorts'],
			enabled: host_ids.length > 0
		};
	});

	return {
		get services(): Service[] {
			return query.data?.items ?? [];
		}
	};
}

/**
 * A host display context: the services and addresses `HostDisplay` picks a host's own from, plus
 * whatever the surface adds (a disabled reason, a tag picker). Every host picker and card builds
 * its context here, so they all show the same icon, tags and addresses.
 */
export function hostDisplayContext(
	ipAddresses: IPAddress[],
	services: Service[],
	extra: HostDisplayContext = {}
): HostDisplayContext {
	return { services, ipAddresses, ...extra };
}

export interface HostPickerOptions {
	/** Omit for a picker that spans every site the user can see. */
	siteId?: string;
	/** Set false to hold the fetch until the picker is shown. */
	enabled?: boolean;
}

/**
 * Everything a host RichSelect or ListManager needs: a page of hosts ordered by title, searched
 * and paged on the server, with the services and addresses each row shows.
 *
 * Pass `options`, `onSearchChange`, `onLoadMore`, `hasMore` and `loading` to the select one by one
 * (spreading would read the getters once and freeze them), and build each row's context with
 * `context(extra)`.
 */
export function useHostPicker(getOptions: () => HostPickerOptions = () => ({})) {
	let search = $state('');

	const query = useHostPickerQuery(() => {
		const { siteId, enabled } = getOptions();
		return { site_id: siteId, enabled, search };
	});

	// Offset paging over a list that can change between fetches can repeat a host across pages,
	// and the select keys its rows by id.
	const hosts = $derived.by(() => {
		const seen: Record<string, true> = {};
		return (query.data?.pages ?? [])
			.flatMap((page) => page.items)
			.filter((host) => {
				if (seen[host.id]) return false;
				seen[host.id] = true;
				return true;
			});
	});

	const services = useHostServices(() => hosts.map((host) => host.id));
	const addresses = $derived(hosts.flatMap((host) => host.ip_addresses));

	return {
		get options(): HostWithAddresses[] {
			return hosts;
		},
		get hasMore(): boolean {
			return query.hasNextPage;
		},
		get loading(): boolean {
			return query.isFetching;
		},
		onSearchChange(text: string) {
			search = text;
		},
		onLoadMore() {
			if (query.hasNextPage && !query.isFetchingNextPage) query.fetchNextPage();
		},
		/** Every row's context: `HostDisplay` picks out the services and addresses of the host it draws. */
		context(extra: HostDisplayContext = {}): HostDisplayContext {
			return hostDisplayContext(addresses, services.services, extra);
		}
	};
}

import type { RenderableTopology } from '../types/base';
import type { Node } from '@xyflow/svelte';
import { v4 as uuidv4 } from 'uuid';

const HOST_IDS = [uuidv4(), uuidv4(), uuidv4()];
const SERVICE_IDS = [uuidv4(), uuidv4(), uuidv4()];

export const TUTORIAL_SERVICES = [
	{
		id: SERVICE_IDS[0],
		label: 'Caddy',
		hostId: HOST_IDS[0],
		hostName: 'web-proxy-01',
		serviceDefinition: 'Caddy'
	},
	{
		id: SERVICE_IDS[1],
		label: 'PostgreSQL',
		hostId: HOST_IDS[1],
		hostName: 'db-primary-01',
		serviceDefinition: 'PostgreSQL'
	},
	{
		id: SERVICE_IDS[2],
		label: 'Redis',
		hostId: HOST_IDS[2],
		hostName: 'cache-01',
		serviceDefinition: 'Redis'
	}
];

export const TUTORIAL_TOPOLOGY: RenderableTopology = {
	id: uuidv4(),
	name: 'Tutorial',
	network_id: uuidv4(),
	created_at: new Date().toISOString(),
	updated_at: new Date().toISOString(),
	nodes: [],
	edges: [],
	dependencies: [],
	entity_tags: [],
	bindings: [],
	ports: [],
	ip_addresses: [],
	interfaces: [],
	neighbours: [],
	candidates: [],
	vlans: [],
	options: {
		request: { element_rules: [], container_rules: [] },
		local: {
			hide_edge_types: [],
			edge_color_mode: 'ByType',
			hide_ports: false,
			expand_level: 'ContainersExpanded'
		}
	},
	hosts: TUTORIAL_SERVICES.map((n) => ({
		id: n.hostId,
		name: n.hostName,
		hostname: null,
		description: null,
		hidden: false,
		chassis_id: null,
		management_url: null,
		manufacturer: null,
		model: null,
		os: null,
		serial: null,
		credential_assignments: [],
		network_id: '',
		tags: [],
		source: { type: 'Manual' },
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString()
	})),
	services: TUTORIAL_SERVICES.map((n) => ({
		id: n.id,
		name: n.label,
		host_id: n.hostId,
		network_id: '',
		position: 0,
		service_definition: n.serviceDefinition,
		source: { type: 'Manual' },
		bindings: [],
		tags: [],
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString()
	})),
	subnets: []
} as unknown as RenderableTopology;

// Xyflow nodes positioned in a triangle for the mini topology viewer
const POSITIONS = [
	{ x: 150, y: 0 },
	{ x: 0, y: 130 },
	{ x: 300, y: 130 }
];

export const TUTORIAL_XYFLOW_NODES: Node[] = TUTORIAL_SERVICES.map((n, i) => ({
	id: n.id,
	position: POSITIONS[i],
	width: 150,
	height: 80,
	data: {
		id: n.id,
		node_type: 'Element',
		element_type: 'Service',
		host_id: n.hostId,
		header: n.hostName,
		position: POSITIONS[i],
		size: { x: 150, y: 80 }
	},
	type: 'Element'
}));

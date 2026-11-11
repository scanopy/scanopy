import { describe, it, expect } from 'vitest';
import { dependencyOrder } from '#lib/features/topology/layout/child-order.ts';

const edge = (source: string, target: string) => ({ source, target });

describe('dependencyOrder', () => {
	it('puts a chain listed out of order in edge direction', () => {
		const order = dependencyOrder(
			['mariadb', 'tomcat', 'cloudflared', 'haproxy'],
			[edge('tomcat', 'mariadb'), edge('cloudflared', 'haproxy'), edge('haproxy', 'tomcat')]
		);
		expect(order).toEqual(['cloudflared', 'haproxy', 'tomcat', 'mariadb']);
	});

	it('puts a hub first and its spokes in the order their edges arrive', () => {
		const order = dependencyOrder(
			['uptime-kuma', 'grafana', 'prometheus'],
			[edge('prometheus', 'grafana'), edge('prometheus', 'uptime-kuma')]
		);
		expect(order).toEqual(['prometheus', 'grafana', 'uptime-kuma']);
	});

	it('keeps each connected group contiguous', () => {
		const order = dependencyOrder(
			['a1', 'b1', 'a2', 'b2', 'a3', 'b3'],
			[edge('a1', 'a2'), edge('b1', 'b2'), edge('a2', 'a3'), edge('b2', 'b3')]
		);
		expect(order).toEqual(['a1', 'a2', 'a3', 'b1', 'b2', 'b3']);
	});

	it('puts children no edge touches last, in their given order', () => {
		const order = dependencyOrder(['redis', 'b', 'podman', 'a'], [edge('a', 'b')]);
		expect(order).toEqual(['a', 'b', 'redis', 'podman']);
	});

	it('orders every child exactly once through a cycle', () => {
		const ids = ['x', 'y', 'z', 'w'];
		const order = dependencyOrder(ids, [
			edge('x', 'y'),
			edge('y', 'z'),
			edge('z', 'x'),
			edge('z', 'w')
		]);
		expect([...order].sort()).toEqual([...ids].sort());
		expect(order.indexOf('z')).toBeLessThan(order.indexOf('w'));
	});

	it('ignores edges to anything outside the container', () => {
		const order = dependencyOrder(['b', 'a'], [edge('outside', 'b'), edge('a', 'b')]);
		expect(order).toEqual(['a', 'b']);
	});

	it('gives the same order for linked children whatever order they are passed in', () => {
		const edges = [edge('p', 'q'), edge('q', 'r'), edge('p', 's')];
		const first = dependencyOrder(['p', 'q', 'r', 's'], edges);
		const second = dependencyOrder(['s', 'r', 'q', 'p'], edges);
		expect(second).toEqual(first);
	});
});

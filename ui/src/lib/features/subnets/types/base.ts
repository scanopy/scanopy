import type { components, operations } from '#lib/api/schema';

// Re-export generated types
export type Subnet = components['schemas']['Subnet'];
export type SubnetType = components['schemas']['SubnetType'];
export type SubnetResponse = components['schemas']['SubnetResponse'];
/** What a person is creating a subnet for: the create flow's first answer. */
export type SubnetCreatePurpose = NonNullable<
	NonNullable<operations['create_subnet']['parameters']['query']>['purpose']
>;

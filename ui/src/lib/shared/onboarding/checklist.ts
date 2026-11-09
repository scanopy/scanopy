import type { components } from '$lib/api/schema';
import { openModal } from '$lib/shared/stores/modal-registry';
import { trackEvent } from '$lib/shared/utils/analytics';
import {
	gettingStarted_stepAccountDescription,
	gettingStarted_stepAccountLabel,
	gettingStarted_stepDaemonDescription,
	gettingStarted_stepDaemonLabel,
	gettingStarted_stepDiscoveryDescription,
	gettingStarted_stepDiscoveryLabel,
	gettingStarted_stepTopologyDescription,
	gettingStarted_stepTopologyLabel
} from '$lib/paraglide/messages';

type SetupItemType = components['schemas']['SetupItemDiscriminants'];

export interface ChecklistStep {
	id: string;
	/** The setup item that completes the step. `null` for the account step, done at signup. */
	milestone: SetupItemType | null;
	prerequisite: SetupItemType | null;
	label: () => string;
	description: () => string;
	actionTab: string;
	actionModal?: string;
}

export const CHECKLIST_STEPS: ChecklistStep[] = [
	{
		id: 'account',
		milestone: null,
		prerequisite: null,
		label: () => gettingStarted_stepAccountLabel(),
		description: () => gettingStarted_stepAccountDescription(),
		actionTab: 'home'
	},
	{
		id: 'daemon',
		milestone: 'InstallDaemon',
		prerequisite: null,
		label: () => gettingStarted_stepDaemonLabel(),
		description: () => gettingStarted_stepDaemonDescription(),
		actionTab: 'daemons',
		actionModal: 'create-daemon'
	},
	{
		id: 'discovery',
		milestone: 'RunFirstScan',
		prerequisite: 'InstallDaemon',
		label: () => gettingStarted_stepDiscoveryLabel(),
		description: () => gettingStarted_stepDiscoveryDescription(),
		actionTab: 'discovery-scans'
	},
	{
		id: 'topology',
		milestone: 'OpenTopology',
		prerequisite: 'RunFirstScan',
		label: () => gettingStarted_stepTopologyLabel(),
		description: () => gettingStarted_stepTopologyDescription(),
		actionTab: 'topology'
	}
];

export function isStepComplete(step: ChecklistStep, setup: SetupItemType[]): boolean {
	return step.milestone === null || setup.includes(step.milestone);
}

export function isStepEnabled(step: ChecklistStep, setup: SetupItemType[]): boolean {
	if (step.prerequisite === null) return true;
	return setup.includes(step.prerequisite);
}

export function getCompletedCount(setup: SetupItemType[]): number {
	return CHECKLIST_STEPS.filter((s) => isStepComplete(s, setup)).length;
}

export function isAllComplete(setup: SetupItemType[]): boolean {
	return CHECKLIST_STEPS.every((s) => isStepComplete(s, setup));
}

/**
 * Whether the homepage shows the pre-daemon prompt in the checklist's slot. The
 * checklist carries its own daemon step, so the prompt only fills in once the
 * checklist is dismissed.
 */
export function homeDaemonPrompt(daemonConnected: boolean, checklistDismissed: boolean): boolean {
	return checklistDismissed && !daemonConnected;
}

export function executeStepAction(step: ChecklistStep, navigate: (tab: string) => void): void {
	navigate(step.actionTab);
	if (step.actionModal) {
		openModal(step.actionModal);
	}
}

export function trackChecklistStepClicked(stepId: string, source: 'home' | 'sidebar'): void {
	trackEvent('checklist_step_clicked', { step_id: stepId, source });
}

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

type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

export interface ChecklistStep {
	id: string;
	milestone: OnboardingOperation;
	prerequisite: OnboardingOperation | null;
	label: () => string;
	description: () => string;
	actionTab: string;
	actionModal?: string;
}

export const CHECKLIST_STEPS: ChecklistStep[] = [
	{
		id: 'account',
		milestone: 'OrgCreated',
		prerequisite: null,
		label: () => gettingStarted_stepAccountLabel(),
		description: () => gettingStarted_stepAccountDescription(),
		actionTab: 'home'
	},
	{
		id: 'daemon',
		milestone: 'FirstDaemonRegistered',
		prerequisite: 'OrgCreated',
		label: () => gettingStarted_stepDaemonLabel(),
		description: () => gettingStarted_stepDaemonDescription(),
		actionTab: 'daemons',
		actionModal: 'create-daemon'
	},
	{
		id: 'discovery',
		milestone: 'FirstDiscoveryCompleted',
		prerequisite: 'FirstDaemonRegistered',
		label: () => gettingStarted_stepDiscoveryLabel(),
		description: () => gettingStarted_stepDiscoveryDescription(),
		actionTab: 'discovery-scans'
	},
	{
		id: 'topology',
		milestone: 'FirstTopologyRebuild',
		prerequisite: 'FirstDiscoveryCompleted',
		label: () => gettingStarted_stepTopologyLabel(),
		description: () => gettingStarted_stepTopologyDescription(),
		actionTab: 'topology'
	}
];

export function isStepComplete(step: ChecklistStep, onboarding: OnboardingOperation[]): boolean {
	return onboarding.includes(step.milestone);
}

export function isStepEnabled(step: ChecklistStep, onboarding: OnboardingOperation[]): boolean {
	if (step.prerequisite === null) return true;
	return onboarding.includes(step.prerequisite);
}

export function getCompletedCount(onboarding: OnboardingOperation[]): number {
	return CHECKLIST_STEPS.filter((s) => onboarding.includes(s.milestone)).length;
}

export function isAllComplete(onboarding: OnboardingOperation[]): boolean {
	return CHECKLIST_STEPS.every((s) => onboarding.includes(s.milestone));
}

export function hasDaemon(onboarding: OnboardingOperation[]): boolean {
	return onboarding.includes('FirstDaemonRegistered');
}

/**
 * Whether the homepage shows the pre-daemon prompt in the checklist's slot. The
 * checklist carries its own daemon step, so the prompt only fills in once the
 * checklist is dismissed.
 */
export function homeDaemonPrompt(
	onboarding: OnboardingOperation[],
	checklistDismissed: boolean
): boolean {
	return checklistDismissed && !hasDaemon(onboarding);
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

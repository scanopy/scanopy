var TEST_PLANS = [
{
  "branch": "fix/disabled-button-ux",
  "tests": [
    {
      "id": "host-delete-with-daemon",
      "category": "Delete Buttons",
      "description": "Host delete button is clickable when host has a daemon, and shows error toast",
      "steps": [
        "Navigate to the Hosts page",
        "Find the host that has an associated daemon",
        "Click the delete button on the host card",
        "Confirm the delete in the confirmation dialog"
      ],
      "setup": "Ensure at least one host exists that has an associated daemon installed on it.",
      "expected": "Delete button is not grayed out. After confirming delete, a toast error appears explaining the host has an associated daemon and to delete the daemon first.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "ifentries-tab-empty-state",
      "category": "Host Editor Tabs",
      "description": "ifEntries tab is visible and shows empty state with ListConfigEditor layout when host has no SNMP data",
      "steps": [
        "Navigate to the Hosts page",
        "Click edit on a host that has no ifEntries (no SNMP data)",
        "Click the ifEntries tab in the host editor",
        "Verify the two-panel layout is shown with empty list on left and informative message on right"
      ],
      "setup": "Ensure at least one host exists that has no ifEntry records (e.g., a host that was not discovered via SNMP).",
      "expected": "ifEntries tab is visible and not grayed out. Clicking it shows the ListConfigEditor two-panel layout: left panel shows empty list message, right panel shows 'No SNMP Interface Entries' with subtitle explaining when entries will appear.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "virtualization-tab-empty-state",
      "category": "Host Editor Tabs",
      "description": "Virtualization tab is visible and shows empty state with ListConfigEditor layout when host has no virtualization services",
      "steps": [
        "Navigate to the Hosts page",
        "Click edit on a host that has no Docker or hypervisor services",
        "Verify the Virtualization tab is visible in the tab bar",
        "Click the Virtualization tab",
        "Verify the two-panel layout is shown with empty list on left and informative message on right"
      ],
      "setup": "Ensure at least one host exists that has no services with manages_virtualization set.",
      "expected": "Virtualization tab is visible (not hidden). Clicking it shows the ListConfigEditor two-panel layout: left panel shows empty list message, right panel shows 'No Virtualization Services' with subtitle explaining when they will appear.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/loopback-interface-handling",
  "tests": []
}
,
{
  "branch": "fix/list-config-editor",
  "tests": []
}
,
{
  "branch": "fix/daemon-404-errors",
  "tests": []
}
,
{
  "branch": "fix/subnet-tags-multi-target-ips",
  "tests": []
}
,
{
  "branch": "test/credential-seed-e2e",
  "tests": []
}
,
{
  "branch": "fix/entity-deletion-ux",
  "tests": []
}
,
{
  "branch": "fix/docker-service-reconciliation",
  "tests": []
}
,
{
  "branch": "tooling/docker-proxy-test-env",
  "tests": []
}
,
{
  "branch": "feat/discovery-scan-settings",
  "tests": []
}
,
{
  "branch": "fix/credential-service-port-binding",
  "tests": []
}
,
{
  "branch": "fix/non-interfaced-host-creation",
  "tests": []
}
,
{
  "branch": "fix/topology-creation-422",
  "tests": []
}
,
{
  "branch": "docs/unified-discovery-migration",
  "tests": []
}
,
{
  "branch": "fix/stalled-session-orphan",
  "tests": []
}
];

// Writes tools/testing/test-plans.js from the TEST_PLAN.json paths given as arguments,
// leaving out every test already marked passed in the TEST_RESULTS.json next to its plan.
// A plan with nothing left to test is dropped.
//
// TEST_RESULTS.json comes in two shapes: the runner's export, keyed by branch
// ({ "<branch>": { tests: [...] } }), and a plan-shaped file ({ branch, tests: [...] }).
const fs = require("fs");
const path = require("path");

function passedIds(planPath, branch) {
  const resultsPath = path.join(path.dirname(planPath), "TEST_RESULTS.json");
  if (!fs.existsSync(resultsPath)) return new Set();
  let results;
  try {
    results = JSON.parse(fs.readFileSync(resultsPath, "utf8"));
  } catch (e) {
    console.warn(`  Ignoring unreadable ${resultsPath}: ${e.message}`);
    return new Set();
  }
  const tests =
    results.branch === branch ? results.tests : results[branch]?.tests;
  return new Set(
    (tests || []).filter((t) => t.status === "passed").map((t) => t.id),
  );
}

const plans = [];
for (const planPath of process.argv.slice(2)) {
  const plan = JSON.parse(fs.readFileSync(planPath, "utf8"));
  const passed = passedIds(planPath, plan.branch);
  const tests = plan.tests.filter((t) => !passed.has(t.id));
  const skipped = plan.tests.length - tests.length;
  const note = skipped ? ` (${skipped} passed, left out)` : "";
  if (tests.length === 0) {
    console.log(`  Skipped: ${planPath}, every test passed`);
    continue;
  }
  plans.push({ ...plan, tests });
  console.log(`  Found: ${planPath}${note}`);
}

fs.writeFileSync(
  path.join(__dirname, "test-plans.js"),
  `var TEST_PLANS = ${JSON.stringify(plans, null, 2)};\n`,
);

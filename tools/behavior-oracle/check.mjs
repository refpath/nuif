import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { isDeepStrictEqual } from "node:util";

const [fixturePath, staticReportPath, reportPath] = process.argv.slice(2);
if (!fixturePath || !staticReportPath || !reportPath) {
  throw new Error(
    "usage: node check.mjs <fixture.json> <static-report.json> <report.json>",
  );
}

const fixtureBytes = await fs.readFile(fixturePath);
const fixture = JSON.parse(fixtureBytes);
const staticReport = JSON.parse(await fs.readFile(staticReportPath, "utf8"));
if (
  fixture.profile !== "nuif-behavior-state-machine-0"
  || staticReport.status !== "passed"
) {
  throw new Error("behavior oracle inputs do not identify a passing bounded profile");
}

const effects = {
  visibility: { capability: "effect.visibility", valueType: "boolean" },
  announcement: { capability: "effect.announcement", valueType: "string" },
};
const activatableRoles = new Set(["button", "checkbox", "radio", "switch"]);

class BehaviorOracleError extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
  }
}

function reject(code, message) {
  throw new BehaviorOracleError(code, message);
}

function requireExactKeys(value, keys, subject) {
  if (
    value === null
    || typeof value !== "object"
    || Array.isArray(value)
    || !isDeepStrictEqual(Object.keys(value).sort(), [...keys].sort())
  ) {
    reject("unknown-or-missing-field", subject);
  }
}

function requireIdentifier(value, subject) {
  if (
    typeof value !== "string"
    || Buffer.byteLength(value) > 128
    || !/^[a-z0-9][a-z0-9_.:-]*$/.test(value)
  ) {
    reject("invalid-identifier", subject);
  }
}

function validateFixture(input) {
  const { program } = input;
  requireExactKeys(
    program,
    ["schema_version", "profile", "capabilities", "initial_state", "variables", "states"],
    "program",
  );
  if (
    program.schema_version !== 1
    || program.profile !== "nuif-behavior-state-machine-0"
  ) {
    reject("unsupported-profile", "unsupported behavior header");
  }
  const entities = new Map(input.document.entities.map((entity) => [entity.id, entity]));
  const stateEntries = Object.entries(program.states);
  if (stateEntries.length === 0 || stateEntries.length > 128) {
    reject("state-limit", "invalid state count");
  }
  if (!Object.hasOwn(program.states, program.initial_state)) {
    reject("missing-initial-state", "initial state does not exist");
  }
  requireIdentifier(program.initial_state, "initial state");
  if (Object.keys(program.variables).length > 128) {
    reject("variable-limit", "too many variables");
  }
  if (Object.keys(program.capabilities).length > 64) {
    reject("capability-limit", "too many capabilities");
  }
  for (const [variable, value] of Object.entries(program.variables)) {
    requireIdentifier(variable, "variable");
    validateValue(value);
  }
  for (const [capability, policy] of Object.entries(program.capabilities)) {
    requireIdentifier(capability, "capability");
    if (!["effect.visibility", "effect.announcement"].includes(capability)) {
      reject("unsupported-capability", capability);
    }
    if (!["required", "optional_noop"].includes(policy)) {
      reject("unsupported-capability-policy", policy);
    }
  }
  let transitionCount = 0;
  let actionCount = 0;
  const transitionIds = new Set();
  for (const [stateName, state] of stateEntries) {
    requireIdentifier(stateName, "state");
    requireExactKeys(state, ["transitions"], `state ${stateName}`);
    transitionCount += state.transitions.length;
    for (const transition of state.transitions) {
      requireExactKeys(
        transition,
        ["id", "event", "guard", "target_state", "actions"],
        `transition ${transition.id}`,
      );
      requireIdentifier(transition.id, "transition");
      requireIdentifier(transition.target_state, "target state");
      if (transitionIds.has(transition.id)) {
        reject("duplicate-transition", transition.id);
      }
      transitionIds.add(transition.id);
      if (!Object.hasOwn(program.states, transition.target_state)) {
        reject("missing-target-state", transition.target_state);
      }
      const source = entities.get(transition.event.source);
      requireExactKeys(transition.event, ["kind", "source"], "event");
      if (
        transition.event.kind !== "activate"
        || !source
        || !activatableRoles.has(source.role)
      ) {
        reject("invalid-event-source", transition.id);
      }
      if (transition.guard !== null) {
        requireExactKeys(transition.guard, ["type", "variable", "value"], "guard");
        if (transition.guard.type !== "equals") {
          reject("unsupported-guard", transition.guard.type);
        }
        requireIdentifier(transition.guard.variable, "guard variable");
        validateVariableValue(program, transition.guard.variable, transition.guard.value);
      }
      if (transition.actions.length > 64) {
        reject("action-limit", transition.id);
      }
      actionCount += transition.actions.length;
      for (const action of transition.actions) {
        validateAction(program, entities, action);
      }
    }
  }
  if (transitionCount > 1024 || actionCount > 4096) {
    reject("graph-limit", "behavior graph is too large");
  }
  const reachable = new Set([program.initial_state]);
  const pending = [program.initial_state];
  while (pending.length > 0) {
    const state = pending.shift();
    for (const transition of program.states[state].transitions) {
      if (!reachable.has(transition.target_state)) {
        reachable.add(transition.target_state);
        pending.push(transition.target_state);
      }
    }
  }
  if (stateEntries.some(([state]) => !reachable.has(state))) {
    reject("unreachable-state", "state is unreachable");
  }
  if (input.events.length > 4096) {
    reject("event-limit", "event sequence is too large");
  }
  for (const event of input.events) {
    requireExactKeys(event, ["kind", "source"], "external event");
    const source = entities.get(event.source);
    if (event.kind !== "activate" || !source || !activatableRoles.has(source.role)) {
      reject("invalid-external-event", event.source);
    }
  }
}

function validateVariableValue(program, variable, value) {
  const initial = program.variables[variable];
  if (!initial || initial.type !== value.type) {
    reject("variable-type", variable);
  }
  validateValue(value);
}

function validateValue(value) {
  requireExactKeys(value, ["type", "value"], "value");
  if (value.type === "boolean" && typeof value.value === "boolean") return;
  if (
    value.type === "string"
    && typeof value.value === "string"
    && Buffer.byteLength(value.value) <= 4096
  ) return;
  reject("unsupported-value", value.type);
}

function validateAction(program, entities, action) {
  if (action.type === "set") {
    requireExactKeys(action, ["type", "variable", "value"], "set action");
    requireIdentifier(action.variable, "action variable");
    validateVariableValue(program, action.variable, action.value);
    return;
  }
  if (action.type === "toggle_boolean") {
    requireExactKeys(action, ["type", "variable"], "toggle action");
    requireIdentifier(action.variable, "action variable");
    if (program.variables[action.variable]?.type !== "boolean") {
      reject("variable-type", action.variable);
    }
    return;
  }
  if (action.type !== "emit" || !Object.hasOwn(effects, action.effect)) {
    reject("unsupported-action", action.type);
  }
  requireExactKeys(action, ["type", "effect", "target", "value"], "emit action");
  const effect = effects[action.effect];
  if (!Object.hasOwn(program.capabilities, effect.capability)) {
    reject("undeclared-capability", effect.capability);
  }
  if (!entities.has(action.target) || action.value.type !== effect.valueType) {
    reject("invalid-effect", action.effect);
  }
  validateValue(action.value);
}

function execute(program, events, supportedList) {
  const supported = new Set(supportedList);
  for (const [capability, policy] of Object.entries(program.capabilities)) {
    if (policy === "required" && !supported.has(capability)) {
      reject("missing-required-capability", capability);
    }
  }
  let activeState = program.initial_state;
  const variables = structuredClone(program.variables);
  const traces = [];
  for (const event of events) {
    const fromState = activeState;
    const transition = program.states[fromState].transitions.find((candidate) =>
      isDeepStrictEqual(candidate.event, event)
      && (candidate.guard === null
        || isDeepStrictEqual(variables[candidate.guard.variable], candidate.guard.value))
    );
    const emitted = [];
    const skipped = [];
    if (transition) {
      for (const action of transition.actions) {
        if (action.type === "set") {
          variables[action.variable] = structuredClone(action.value);
        } else if (action.type === "toggle_boolean") {
          variables[action.variable].value = !variables[action.variable].value;
        } else {
          const capability = effects[action.effect].capability;
          if (supported.has(capability)) {
            emitted.push({
              kind: action.effect,
              target: action.target,
              value: structuredClone(action.value),
            });
          } else {
            skipped.push({
              capability,
              kind: action.effect,
              target: action.target,
            });
          }
        }
      }
      activeState = transition.target_state;
    }
    traces.push({
      event: structuredClone(event),
      from_state: fromState,
      transition: transition?.id ?? null,
      to_state: activeState,
      variables: structuredClone(variables),
      effects: emitted,
      skipped_optional: skipped,
    });
  }
  return {
    schema_version: 1,
    profile: "nuif-behavior-state-machine-0",
    traces,
    final_state: activeState,
    variables,
  };
}

const mismatches = [];
try {
  validateFixture(fixture);
} catch (error) {
  mismatches.push({ category: "foreign-validation", code: error.code, message: error.message });
}
const runs = [];
if (mismatches.length === 0) {
  for (const run of fixture.runs) {
    const actual = execute(fixture.program, fixture.events, run.capabilities);
    try {
      assert.deepStrictEqual(actual, run.expected);
      runs.push({ name: run.name, status: "passed", actual });
    } catch (error) {
      mismatches.push({ category: "trace-mismatch", run: run.name, message: error.message });
      runs.push({ name: run.name, status: "failed", actual });
    }
  }
  try {
    execute(
      fixture.program,
      fixture.events,
      fixture.missing_required.capabilities,
    );
    mismatches.push({ category: "missing-required-capability-was-accepted" });
  } catch (error) {
    if (error.code !== fixture.missing_required.expected_error) {
      mismatches.push({
        category: "required-capability-error",
        expected: fixture.missing_required.expected_error,
        observed: error.code,
      });
    }
  }
}

const passed = mismatches.length === 0;
const report = {
  schema_version: 1,
  experiment: "nuif:experiment:behavior-portability",
  status: passed ? "passed" : "failed",
  profile: fixture.profile,
  oracle: {
    name: "independent-node-state-machine",
    node: process.version,
    operating_system: `${os.platform()} ${os.release()}`,
    architecture: os.arch(),
  },
  fixture: {
    path: fixturePath,
    sha256: crypto.createHash("sha256").update(fixtureBytes).digest("hex"),
    events: fixture.events.length,
    states: Object.keys(fixture.program.states).length,
  },
  runs,
  mismatches,
  non_claims: [
    "the JavaScript oracle is a second profile implementation, not a browser or native UI adapter",
    "the profile excludes timers internal events networking navigation animation and arbitrary code",
    "trace equivalence does not establish visual or assistive-technology behavior equivalence",
  ],
};
await fs.mkdir(path.dirname(reportPath), { recursive: true });
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
if (!passed) {
  throw new Error(`behavior oracle failed; inspect ${reportPath}`);
}
console.log(
  `behavior oracle: ${runs.length} capability runs, ${fixture.events.length} events, status passed`,
);

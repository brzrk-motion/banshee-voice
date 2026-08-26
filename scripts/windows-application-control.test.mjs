import assert from "node:assert/strict";
import test from "node:test";

import {
  classifySmartAppControlQuery,
  parseSmartAppControlState,
} from "./windows-application-control.mjs";

const queryResult = (value) => ({
  status: 0,
  stdout: `    VerifiedAndReputablePolicyState    REG_DWORD    ${value}\r\n`,
});

test("parses hexadecimal and decimal policy values", () => {
  assert.equal(parseSmartAppControlState(queryResult("0x1").stdout), 1);
  assert.equal(parseSmartAppControlState(queryResult("2").stdout), 2);
});

test("allows Off and Evaluation policy states", () => {
  assert.equal(classifySmartAppControlQuery(queryResult("0x0")), "allowed");
  assert.equal(classifySmartAppControlQuery(queryResult("0x2")), "allowed");
});

test("rejects the Enforcement policy state", () => {
  assert.equal(classifySmartAppControlQuery(queryResult("0x1")), "enforced");
});

test("treats failed and unrecognized queries as unavailable", () => {
  assert.equal(
    classifySmartAppControlQuery({ status: 1, stdout: "", error: undefined }),
    "unavailable",
  );
  assert.equal(
    classifySmartAppControlQuery({ status: 0, stdout: "unexpected output" }),
    "unavailable",
  );
});

"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");

const {
  IMMUTABLE_RELEASE_ASSETS,
  RELEASE_ASSET_GUARD_STATE,
  evaluateReleaseAssetGuard,
} = require("./release_asset_guard");

test("marks guard as complete and skips build/upload when all immutable assets already exist", () => {
  const result = evaluateReleaseAssetGuard({
    existingAssetNames: ["cmux-macos.dmg", "appcast.xml", "notes.txt"],
  });

  assert.deepEqual(result.conflicts, IMMUTABLE_RELEASE_ASSETS);
  assert.deepEqual(result.missingImmutableAssets, []);
  assert.equal(result.guardState, RELEASE_ASSET_GUARD_STATE.COMPLETE);
  assert.equal(result.hasPartialConflict, false);
  assert.equal(result.shouldSkipBuildAndUpload, true);
  assert.equal(result.shouldSkipUpload, true);
});

test("marks guard as clear when immutable assets are not present", () => {
  const result = evaluateReleaseAssetGuard({
    existingAssetNames: ["notes.txt", "checksums.txt"],
  });

  assert.deepEqual(result.conflicts, []);
  assert.deepEqual(result.missingImmutableAssets, IMMUTABLE_RELEASE_ASSETS);
  assert.equal(result.guardState, RELEASE_ASSET_GUARD_STATE.CLEAR);
  assert.equal(result.hasPartialConflict, false);
  assert.equal(result.shouldSkipBuildAndUpload, false);
  assert.equal(result.shouldSkipUpload, false);
});

test("marks guard as partial when only some immutable assets exist", () => {
  const result = evaluateReleaseAssetGuard({
    existingAssetNames: ["appcast.xml"],
  });

  assert.deepEqual(result.conflicts, ["appcast.xml"]);
  assert.deepEqual(result.missingImmutableAssets, ["cmux-macos.dmg"]);
  assert.equal(result.guardState, RELEASE_ASSET_GUARD_STATE.PARTIAL);
  assert.equal(result.hasPartialConflict, true);
  assert.equal(result.shouldSkipBuildAndUpload, false);
  assert.equal(result.shouldSkipUpload, false);
});

test("supports custom immutable asset sets for Linux release artifacts", () => {
  const result = evaluateReleaseAssetGuard({
    existingAssetNames: ["cmux-linux-x86_64.tar.gz", "notes.txt"],
    immutableAssetNames: ["cmux-linux-x86_64.tar.gz", "cmux-linux-x86_64.AppImage"],
  });

  assert.deepEqual(result.conflicts, ["cmux-linux-x86_64.tar.gz"]);
  assert.deepEqual(result.missingImmutableAssets, ["cmux-linux-x86_64.AppImage"]);
  assert.equal(result.guardState, RELEASE_ASSET_GUARD_STATE.PARTIAL);
  assert.equal(result.hasPartialConflict, true);
  assert.equal(result.shouldSkipBuildAndUpload, false);
  assert.equal(result.shouldSkipUpload, false);
});

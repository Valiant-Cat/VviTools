import { test } from "node:test";
import assert from "node:assert/strict";
import { createManifest } from "./create-updater-manifest.mjs";

const input = {
  version: "0.2.0", tag: "v0.2.0", repository: "Valiant-Cat/VviTools",
  asset: "VviTools.app.tar.gz", signature: "test-signature",
  publishedAt: "2026-09-15T00:00:00Z",
};

test("通用更新包同时提供两种 macOS 架构", () => {
  const result = createManifest(input);
  assert.equal(result.version, input.version);
  assert.deepEqual(Object.keys(result.platforms), ["darwin-aarch64", "darwin-x86_64"]);
  assert.equal(result.platforms["darwin-aarch64"].signature, input.signature);
  assert.equal(result.platforms["darwin-x86_64"].url,
    "https://github.com/Valiant-Cat/VviTools/releases/download/v0.2.0/VviTools.app.tar.gz");
});

test("拒绝标签不一致、预发布、空签名及无效数据", () => {
  for (const patch of [
    { tag: "v0.1.0" }, { version: "0.2.0-beta.1" }, { signature: " " },
    { asset: "../bad.app.tar.gz" }, { repository: "bad" }, { publishedAt: "bad" },
  ]) {
    assert.throws(() => createManifest({ ...input, ...patch }));
  }
});

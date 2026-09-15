import { readFile, readdir, writeFile } from "node:fs/promises";
import { resolve, basename } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

export function createManifest({ version, tag, repository, asset, signature, notes = "", publishedAt }) {
  if (!/^\d+\.\d+\.\d+$/.test(version) || tag !== `v${version}`) {
    throw new Error("发布标签必须与应用稳定版本一致");
  }
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository)) throw new Error("仓库名称无效");
  if (basename(asset) !== asset || !asset.endsWith(".app.tar.gz")) throw new Error("更新包名称无效");
  if (!signature?.trim()) throw new Error("缺少更新包签名");
  if (!Number.isFinite(Date.parse(publishedAt))) throw new Error("发布时间无效");
  const platform = {
    signature: signature.trim(),
    url: `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(asset)}`,
  };
  return {
    version, notes, pub_date: publishedAt,
    platforms: { "darwin-aarch64": platform, "darwin-x86_64": platform },
  };
}

async function main() {
  const root = fileURLToPath(new URL("../", import.meta.url));
  const config = JSON.parse(await readFile(resolve(root, "src-tauri/tauri.conf.json"), "utf8"));
  const pkg = JSON.parse(await readFile(resolve(root, "package.json"), "utf8"));
  const cargo = JSON.parse(execFileSync("cargo", [
    "metadata", "--no-deps", "--format-version", "1",
    "--manifest-path", resolve(root, "src-tauri/Cargo.toml"),
  ], { encoding: "utf8" }));
  if (pkg.version !== config.version || cargo.packages.find(p => p.name === "vvitools")?.version !== config.version) {
    throw new Error("package.json、Cargo.toml 与 Tauri 版本必须一致");
  }
  const dir = resolve(root, "src-tauri/target/universal-apple-darwin/release/bundle/macos");
  const assets = (await readdir(dir)).filter(name => name.endsWith(".app.tar.gz"));
  if (assets.length !== 1) throw new Error("必须且只能有一个通用架构更新包");
  const manifest = createManifest({
    version: config.version,
    tag: process.env.RELEASE_TAG,
    repository: process.env.GITHUB_REPOSITORY,
    asset: assets[0],
    signature: await readFile(resolve(dir, `${assets[0]}.sig`), "utf8"),
    notes: process.env.RELEASE_NOTES || `VviTools ${config.version}`,
    publishedAt: new Date().toISOString(),
  });
  await writeFile(resolve(dir, "latest.json"), JSON.stringify(manifest, null, 2) + "\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(error => { console.error(error.message); process.exitCode = 1; });
}

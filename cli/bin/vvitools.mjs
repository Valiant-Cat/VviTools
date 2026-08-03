#!/usr/bin/env node

import { existsSync } from "node:fs";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const VERSION = "0.1.0";
const VALID_RUNTIMES = new Set(["node", "shell", "builtin"]);
const USER_RUNTIMES = new Set(["node", "shell"]);
const INPUT_TYPES = new Set(["none", "text"]);
const PLUGIN_CATEGORIES = new Set(["efficiency", "search", "image", "developer", "system"]);

main().catch((error) => {
  console.error(`错误: ${error.message}`);
  process.exit(1);
});

async function main() {
  const rawArgs = process.argv.slice(2);
  const usesPluginSubcommand = rawArgs[0] === "plugin";
  const [command, ...argv] = normalizeArgs(rawArgs);

  if (!command || command === "-h" || command === "--help") {
    printHelp(usesPluginSubcommand);
    return;
  }
  if (command === "-v" || command === "--version") {
    console.log(VERSION);
    return;
  }

  if (command === "create" || command === "init") {
    await createPlugin(argv);
    return;
  }
  if (command === "validate") {
    await validatePluginCommand(argv);
    return;
  }
  if (command === "pack") {
    await packPlugin(argv);
    return;
  }

  throw new Error(`未知命令: ${command}`);
}

function normalizeArgs(args) {
  if (args[0] === "plugin") {
    return args.slice(1);
  }
  return args;
}

function printHelp(pluginSubcommand) {
  if (pluginSubcommand) {
    console.log(pluginHelp("vvitools plugin"));
    return;
  }

  console.log(`vvitools ${VERSION}

用法:
  vvitools plugin create <目录或插件名> [--runtime node|shell] [--name 显示名] [--id 插件ID] [--category 分类]
  vvitools plugin validate <插件目录> [--allow-builtin]
  vvitools plugin pack <插件目录> [--out dist/xxx.zip] [--allow-builtin]

命令:
  plugin      插件开发工具，支持创建、校验和打包插件

示例:
  vvitools plugin create hello-tools --runtime node --name "Hello 工具" --category efficiency
  vvitools plugin validate hello-tools
  vvitools plugin pack hello-tools`);
}

function pluginHelp(commandName) {
  return `${commandName} ${VERSION}

用法:
  ${commandName} create <目录或插件名> [--runtime node|shell] [--name 显示名] [--id 插件ID] [--category 分类]
  ${commandName} validate <插件目录> [--allow-builtin]
  ${commandName} pack <插件目录> [--out dist/xxx.zip] [--allow-builtin]

命令:
  create      创建 VviTools 插件模板
  validate    校验 plugin.json 和入口文件
  pack        校验后打包为 zip，供插件市场或本地安装使用

示例:
  ${commandName} create hello-tools --runtime node --name "Hello 工具" --category efficiency
  ${commandName} validate hello-tools
  ${commandName} pack hello-tools`;
}

async function createPlugin(argv) {
  const { positional, options } = parseArgs(argv);
  const requestedName = positional[0];
  if (!requestedName) {
    throw new Error("create 需要传入插件目录或插件名");
  }

  const runtime = options.runtime ?? "node";
  if (!USER_RUNTIMES.has(runtime)) {
    throw new Error("第三方插件模板只支持 runtime: node 或 shell");
  }

  const targetDir = path.resolve(options.dir ?? requestedName);
  if (existsSync(targetDir)) {
    const entries = await readdir(targetDir);
    if (entries.length > 0) {
      throw new Error(`目标目录不是空目录: ${targetDir}`);
    }
  }

  const slug = toSlug(path.basename(targetDir));
  const category = options.category ?? "efficiency";
  if (!PLUGIN_CATEGORIES.has(category)) {
    throw new Error(`category 必须是以下之一: ${Array.from(PLUGIN_CATEGORIES).join(", ")}`);
  }
  const manifest = {
    id: options.id ?? `dev.vvicat.${slug}`,
    name: options.name ?? toDisplayName(slug),
    version: "0.1.0",
    description: "一个 VviTools 插件。",
    icon: "assets/icon.svg",
    keywords: [slug],
    categories: [category],
    runtime,
    entry: runtime === "node" ? "main.mjs" : "main.sh",
    permissions: [],
    commands: [
      {
        id: `${slug}.run`,
        title: `运行${options.name ?? toDisplayName(slug)}`,
        keyword: slug,
        input: "text",
      },
    ],
  };

  await mkdir(path.join(targetDir, "assets"), { recursive: true });
  await writeJson(path.join(targetDir, "plugin.json"), manifest);
  await writeFile(
    path.join(targetDir, "README.md"),
    `# ${manifest.name}

这是一个 VviTools ${runtime} 插件。

## 本地校验

\`\`\`bash
vvitools plugin validate .
\`\`\`

## 打包

\`\`\`bash
vvitools plugin pack .
\`\`\`
`,
  );
  await writeFile(path.join(targetDir, "assets", "icon.svg"), iconSvg(manifest.name));

  if (runtime === "node") {
    await writeFile(path.join(targetDir, "main.mjs"), nodeTemplate());
  } else {
    await writeFile(path.join(targetDir, "main.sh"), shellTemplate(), { mode: 0o755 });
  }

  await validatePlugin(targetDir, { allowBuiltin: false });
  console.log(`已创建插件模板: ${targetDir}`);
}

async function validatePluginCommand(argv) {
  const { positional, options } = parseArgs(argv);
  const pluginDir = path.resolve(positional[0] ?? ".");
  const manifest = await validatePlugin(pluginDir, {
    allowBuiltin: Boolean(options["allow-builtin"]),
  });
  console.log(`校验通过: ${manifest.name} (${manifest.id})`);
}

async function packPlugin(argv) {
  const { positional, options } = parseArgs(argv);
  const pluginDir = path.resolve(positional[0] ?? ".");
  const manifest = await validatePlugin(pluginDir, {
    allowBuiltin: Boolean(options["allow-builtin"]),
  });

  const defaultName = `${manifest.id}-${manifest.version}.zip`;
  const outPath = path.resolve(options.out ?? path.join(pluginDir, "dist", defaultName));
  await mkdir(path.dirname(outPath), { recursive: true });

  const files = await collectFiles(pluginDir);
  const zipEntries = [];
  for (const relativePath of files) {
    if (relativePath === path.relative(pluginDir, outPath)) {
      continue;
    }
    zipEntries.push({
      name: relativePath.split(path.sep).join("/"),
      data: await readFile(path.join(pluginDir, relativePath)),
    });
  }
  await writeZip(outPath, zipEntries);
  console.log(`已打包插件: ${outPath}`);
}

async function validatePlugin(pluginDir, { allowBuiltin }) {
  const manifestPath = path.join(pluginDir, "plugin.json");
  const manifest = await readJson(manifestPath);
  const errors = [];

  requiredString(manifest, "id", errors);
  requiredString(manifest, "name", errors);
  requiredString(manifest, "version", errors);
  requiredString(manifest, "description", errors);
  requiredString(manifest, "entry", errors);
  if (!VALID_RUNTIMES.has(manifest.runtime)) {
    errors.push("runtime 必须是 node、shell 或 builtin");
  }
  if (manifest.runtime === "builtin" && !allowBuiltin) {
    errors.push("第三方插件不允许使用 builtin runtime；内部校验请加 --allow-builtin");
  }
  if (hasParentSegment(manifest.entry)) {
    errors.push("entry 不能包含上级目录");
  }
  if (manifest.icon && hasParentSegment(manifest.icon)) {
    errors.push("icon 不能包含上级目录");
  }

  const entryPath = path.join(pluginDir, manifest.entry ?? "");
  if (manifest.runtime !== "builtin" && manifest.entry && !existsSync(entryPath)) {
    errors.push(`入口文件不存在: ${manifest.entry}`);
  }

  if (!Array.isArray(manifest.keywords)) {
    errors.push("keywords 必须是数组");
  }
  if (!Array.isArray(manifest.categories) || manifest.categories.length === 0) {
    errors.push(`categories 必须至少包含一个分类: ${Array.from(PLUGIN_CATEGORIES).join(", ")}`);
  } else {
    manifest.categories.forEach((category, index) => {
      if (typeof category !== "string" || category.trim() === "") {
        errors.push(`categories[${index}] 不能为空`);
      } else if (!PLUGIN_CATEGORIES.has(category)) {
        errors.push(`categories[${index}] 不支持: ${category}`);
      }
    });
  }
  if (!Array.isArray(manifest.permissions)) {
    errors.push("permissions 必须是数组");
  }
  if (!Array.isArray(manifest.commands) || manifest.commands.length === 0) {
    errors.push("commands 至少需要包含一个命令");
  } else {
    manifest.commands.forEach((command, index) => {
      requiredOwnString(command, "id", `commands[${index}].id`, errors);
      requiredOwnString(command, "title", `commands[${index}].title`, errors);
      requiredOwnString(command, "keyword", `commands[${index}].keyword`, errors);
      requiredOwnString(command, "input", `commands[${index}].input`, errors);
      if (command.input && !INPUT_TYPES.has(command.input)) {
        errors.push(`commands[${index}].input 当前只建议使用 none 或 text`);
      }
    });
  }

  if (manifest.runtime === "builtin") {
    const builtin = manifest.builtin;
    if (!builtin || typeof builtin !== "object") {
      errors.push("builtin runtime 必须声明 builtin 配置");
    } else {
      requiredOwnString(builtin, "module", "builtin.module", errors);
      requiredOwnString(builtin, "bridge", "builtin.bridge", errors);
      if (!Array.isArray(builtin.host_commands) || builtin.host_commands.length === 0) {
        errors.push("builtin.host_commands 至少需要包含一个宿主命令");
      }
    }
  }

  if (errors.length > 0) {
    throw new Error(`插件校验失败:\n- ${errors.join("\n- ")}`);
  }
  return manifest;
}

function parseArgs(argv) {
  const positional = [];
  const options = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("--")) {
      positional.push(arg);
      continue;
    }

    const [rawKey, inlineValue] = arg.slice(2).split("=", 2);
    const next = argv[index + 1];
    if (inlineValue !== undefined) {
      options[rawKey] = inlineValue;
    } else if (!next || next.startsWith("--")) {
      options[rawKey] = true;
    } else {
      options[rawKey] = next;
      index += 1;
    }
  }

  return { positional, options };
}

async function readJson(filePath) {
  try {
    return JSON.parse(await readFile(filePath, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") {
      throw new Error(`文件不存在: ${filePath}`);
    }
    throw new Error(`JSON 解析失败: ${filePath}\n${error.message}`);
  }
}

async function writeJson(filePath, value) {
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function requiredString(object, keyPath, errors) {
  const value = getPath(object, keyPath);
  if (typeof value !== "string" || value.trim() === "") {
    errors.push(`${keyPath} 不能为空`);
  }
}

function requiredOwnString(object, key, label, errors) {
  const value = object?.[key];
  if (typeof value !== "string" || value.trim() === "") {
    errors.push(`${label} 不能为空`);
  }
}

function getPath(object, keyPath) {
  return keyPath.split(".").reduce((current, key) => current?.[key], object);
}

function hasParentSegment(value) {
  return value.split(/[\\/]/).includes("..");
}

function toSlug(value) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .replace(/-{2,}/g, "-") || "plugin";
}

function toDisplayName(slug) {
  return slug
    .split(/[-_.]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

async function collectFiles(root) {
  const result = [];

  async function walk(current) {
    const entries = await readdir(current, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      const relativePath = path.relative(root, fullPath);
      if (shouldSkip(relativePath, entry.name)) {
        continue;
      }
      if (entry.isDirectory()) {
        await walk(fullPath);
      } else if (entry.isFile()) {
        result.push(relativePath);
      }
    }
  }

  await walk(root);
  result.sort();
  return result;
}

function shouldSkip(relativePath, name) {
  return (
    name === ".DS_Store" ||
    name === "node_modules" ||
    name === ".git" ||
    relativePath === "dist" ||
    relativePath.startsWith(`dist${path.sep}`)
  );
}

async function writeZip(outPath, entries) {
  const chunks = [];
  const central = [];
  let offset = 0;

  for (const entry of entries) {
    const name = Buffer.from(entry.name, "utf8");
    const data = Buffer.from(entry.data);
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0x0800, 6);
    local.writeUInt16LE(0, 8);
    local.writeUInt16LE(0, 10);
    local.writeUInt16LE(0, 12);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(name.length, 26);
    local.writeUInt16LE(0, 28);
    chunks.push(local, name, data);

    const header = Buffer.alloc(46);
    header.writeUInt32LE(0x02014b50, 0);
    header.writeUInt16LE(20, 4);
    header.writeUInt16LE(20, 6);
    header.writeUInt16LE(0x0800, 8);
    header.writeUInt16LE(0, 10);
    header.writeUInt16LE(0, 12);
    header.writeUInt16LE(0, 14);
    header.writeUInt32LE(crc, 16);
    header.writeUInt32LE(data.length, 20);
    header.writeUInt32LE(data.length, 24);
    header.writeUInt16LE(name.length, 28);
    header.writeUInt16LE(0, 30);
    header.writeUInt16LE(0, 32);
    header.writeUInt16LE(0, 34);
    header.writeUInt16LE(0, 36);
    header.writeUInt32LE(0, 38);
    header.writeUInt32LE(offset, 42);
    central.push(header, name);

    offset += local.length + name.length + data.length;
  }

  const centralSize = central.reduce((total, chunk) => total + chunk.length, 0);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(0, 4);
  end.writeUInt16LE(0, 6);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(centralSize, 12);
  end.writeUInt32LE(offset, 16);
  end.writeUInt16LE(0, 20);

  await writeFile(outPath, Buffer.concat([...chunks, ...central, end]));
}

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc = CRC32_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

const CRC32_TABLE = Array.from({ length: 256 }, (_, index) => {
  let value = index;
  for (let bit = 0; bit < 8; bit += 1) {
    value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
  }
  return value >>> 0;
});

function iconSvg(label) {
  const initial = [...label.trim()][0]?.toUpperCase() ?? "V";
  return `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200" role="img">
  <rect width="200" height="200" rx="44" fill="#1f2937"/>
  <circle cx="100" cy="100" r="58" fill="#f2c94c"/>
  <text x="100" y="119" text-anchor="middle" font-family="Arial, sans-serif" font-size="64" font-weight="700" fill="#111827">${escapeXml(initial)}</text>
</svg>
`;
}

function nodeTemplate() {
  return `const rawInput = process.env.VVITOOLS_RPC_INPUT ?? "{}";
const request = JSON.parse(rawInput);
const query = request.params?.query ?? "";

process.stdout.write(JSON.stringify({
  type: "list",
  items: [
    {
      title: query ? \`输入内容: \${query}\` : "Hello VviTools",
      subtitle: "来自 node 插件模板",
      action: query ? { type: "copy", value: query } : undefined
    }
  ]
}));
`;
}

function shellTemplate() {
  return `#!/bin/sh
printf '{"type":"text","text":"Hello VviTools"}'
`;
}

function escapeXml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

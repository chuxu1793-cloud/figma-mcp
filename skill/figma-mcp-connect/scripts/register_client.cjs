#!/usr/bin/env node
// Registers figma-mcp in an MCP client config. Idempotent; backs up before write.
// Usage: node register_client.cjs --client <id> --binary <path> [--name figma] [--config <path>]
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const HOME = os.homedir();

// key: [config path, config shape]
// shape 'mcpServers' -> { mcpServers: { name: { command } } }
// shape 'servers'    -> { servers: { name: { type: 'stdio', command } } }  (VS Code family)
function claudeDesktopConfig() {
  if (process.platform === 'darwin') {
    return path.join(HOME, 'Library', 'Application Support', 'Claude', 'claude_desktop_config.json');
  }
  if (process.platform === 'win32') {
    return path.join(process.env.APPDATA || path.join(HOME, 'AppData', 'Roaming'), 'Claude', 'claude_desktop_config.json');
  }
  return path.join(HOME, '.config', 'Claude', 'claude_desktop_config.json');
}

const CLIENTS = {
  codely: [path.join(HOME, '.codely-cli', 'settings.json'), 'mcpServers'],
  'claude-desktop': [claudeDesktopConfig(), 'mcpServers'],
  'claude-code': [path.join(process.cwd(), '.mcp.json'), 'mcpServers'],
  cursor: [path.join(HOME, '.cursor', 'mcp.json'), 'mcpServers'],
  'cursor-project': [path.join(process.cwd(), '.cursor', 'mcp.json'), 'mcpServers'],
  vscode: [path.join(process.cwd(), '.vscode', 'mcp.json'), 'servers'],
};

function die(reason) {
  console.log('STATUS: error');
  console.log(`REASON: ${reason}`);
  process.exit(1);
}

const args = process.argv.slice(2);
const opt = {};
for (let i = 0; i < args.length; i += 2) {
  if (!args[i].startsWith('--') || args[i + 1] === undefined) die(`bad argument: ${args[i]}`);
  opt[args[i].slice(2)] = args[i + 1];
}

const clientId = opt.client;
const binary = opt.binary;
const name = opt.name || 'figma';

if (!clientId || !CLIENTS[clientId]) {
  die(`--client must be one of: ${Object.keys(CLIENTS).join(', ')}`);
}
if (!binary) die('--binary <path-to-figma-mcp> is required');
if (!fs.existsSync(binary)) die(`binary not found: ${binary}`);

const [defaultPath, shape] = CLIENTS[clientId];
const configPath = opt.config ? path.resolve(opt.config) : defaultPath;

let config = {};
if (fs.existsSync(configPath)) {
  const raw = fs.readFileSync(configPath, 'utf8').trim();
  if (raw) {
    try {
      config = JSON.parse(raw);
    } catch (e) {
      // JSONC (comments / trailing commas) is common in .vscode/mcp.json.
      die(`cannot parse ${configPath} as JSON (${e.message}); edit it manually instead`);
    }
  }
}

const entry =
  shape === 'servers'
    ? { type: 'stdio', command: path.resolve(binary) }
    : { command: path.resolve(binary) };

config[shape] = config[shape] || {};
const before = JSON.stringify(config[shape][name]);
const after = JSON.stringify(entry);

if (before === after) {
  console.log('STATUS: already-registered');
} else {
  if (fs.existsSync(configPath)) {
    fs.copyFileSync(configPath, `${configPath}.bak`);
    console.log(`BACKUP: ${configPath}.bak`);
  }
  config[shape][name] = entry;
  fs.mkdirSync(path.dirname(configPath), { recursive: true });
  fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
  console.log(before === undefined ? 'STATUS: registered' : 'STATUS: updated');
}

console.log(`CONFIG: ${configPath}`);
console.log(`ENTRY: ${shape}.${name} = ${after}`);
console.log(`RELOAD: restart ${clientId} (or its MCP connection) to pick up the change`);

import { execSync } from "node:child_process";

function run(command) {
  return execSync(command, { stdio: ["ignore", "pipe", "pipe"], encoding: "utf8" });
}

function getListeningPidsWindows(port) {
  // Note: `netstat -p tcp` may omit IPv6 listeners (e.g. `[::1]:1420`) on Windows.
  const output = run("netstat -ano");
  const lines = output.split(/\r?\n/);
  const pids = new Set();

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("TCP")) continue;
    if (!trimmed.includes(`:${port}`)) continue;
    if (!/\sLISTENING\s/i.test(trimmed)) continue;

    const parts = trimmed.split(/\s+/);
    const pid = parts.at(-1);
    if (pid && /^\d+$/.test(pid)) pids.add(pid);
  }

  return [...pids];
}

function getListeningPidsPosix(port) {
  try {
    const output = run(`lsof -ti tcp:${port} -sTCP:LISTEN`);
    return output
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

function killPidWindows(pid) {
  run(`taskkill /PID ${pid} /T /F`);
}

function killPidPosix(pid) {
  run(`kill -9 ${pid}`);
}

function main() {
  const port = Number(process.argv[2] ?? "");
  if (!Number.isFinite(port) || port <= 0) {
    console.error("Usage: node scripts/kill-port.mjs <port>");
    process.exit(2);
  }

  if (process.env.SKIP_KILL_PORT === "1") {
    console.log(`[kill-port] SKIP_KILL_PORT=1, skip killing port ${port}`);
    return;
  }

  const isWindows = process.platform === "win32";
  const pids = isWindows ? getListeningPidsWindows(port) : getListeningPidsPosix(port);

  if (pids.length === 0) return;

  for (const pid of pids) {
    try {
      if (isWindows) killPidWindows(pid);
      else killPidPosix(pid);
      console.log(`[kill-port] Killed PID ${pid} on port ${port}`);
    } catch (err) {
      console.error(`[kill-port] Failed to kill PID ${pid} on port ${port}`);
      console.error(String(err?.message ?? err));
      process.exit(1);
    }
  }
}

main();

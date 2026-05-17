#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const assetsDir = path.join(root, "assets");
const timingPath = path.join(assetsDir, "benchmark.json");
const metricsPath = path.join(assetsDir, "package-metrics.json");

const timing = fs.existsSync(timingPath)
  ? JSON.parse(fs.readFileSync(timingPath, "utf8"))
  : {
      platform: "darwin-arm64",
      npmRunAll: 200.86085066666666,
      runAllNow: 150.45159016666665,
      fasterPercent: 25.0966080909691
    };

const metrics = fs.existsSync(metricsPath)
  ? JSON.parse(fs.readFileSync(metricsPath, "utf8"))
  : {
      installed: { npmRunAllNodeModulesKiB: 18000, runAllNowNodeModulesKiB: 552 },
      dependencies: { npmRunAllDirectRuntimeDependencies: 9, runAllNowDirectRuntimeDependencies: 0 },
      memory: { npmRunAllPeakFootprintBytes: 24365672, runAllNowPeakFootprintBytes: 13338232 }
    };

fs.mkdirSync(assetsDir, { recursive: true });

function escapeXml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function bytes(value) {
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  return `${(value / 1024).toFixed(1)} KB`;
}

function kib(value) {
  return value >= 1024 ? `${(value / 1024).toFixed(1)} MB` : `${value} KiB`;
}

function ms(value) {
  return `${Math.round(value)} ms`;
}

function percent(value) {
  return `${value.toFixed(1)}%`;
}

function text(x, y, content, options = {}) {
  const size = options.size ?? 20;
  const weight = options.weight ?? 500;
  const fill = options.fill ?? "#cbd5e1";
  const anchor = options.anchor ? ` text-anchor="${options.anchor}"` : "";
  const family = options.mono ? "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" : "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif";
  return `<text x="${x}" y="${y}" fill="${fill}" font-family="${family}" font-size="${size}" font-weight="${weight}"${anchor}>${escapeXml(content)}</text>`;
}

function pill(x, y, label) {
  const width = label.length * 9 + 34;
  return `<g>
    <rect x="${x}" y="${y}" width="${width}" height="34" rx="17" fill="#0b1220" stroke="#263244"/>
    ${text(x + width / 2, y + 23, label, { size: 12, weight: 800, fill: "#a7b3c8", anchor: "middle" })}
  </g>`;
}

function chart({ x, y, title, subtitle, npmLabel, npmValue, npmDisplay, runValue, runDisplay, delta, max }) {
  const width = 542;
  const height = 280;
  const chartX = x + 34;
  const chartY = y + 132;
  const chartWidth = width - 68;
  const barHeight = 18;
  const gap = 54;
  const safeMax = max || Math.max(npmValue, runValue, 1);
  const npmWidth = Math.max(2, (npmValue / safeMax) * chartWidth);
  const runWidth = Math.max(runValue === 0 ? 2 : 2, (runValue / safeMax) * chartWidth);

  return `<g>
    <rect x="${x}" y="${y}" width="${width}" height="${height}" rx="26" fill="#0b1020" stroke="#1f2a3d"/>
    ${text(x + 34, y + 48, title, { size: 26, weight: 850, fill: "#f8fafc" })}
    ${text(x + 34, y + 78, subtitle, { size: 15, weight: 500, fill: "#94a3b8" })}
    ${text(x + 34, y + 108, delta, { size: 16, weight: 800, fill: "#6ee7b7" })}

    ${text(chartX, chartY - 12, npmLabel, { size: 14, weight: 700, fill: "#e2e8f0" })}
    ${text(chartX + chartWidth, chartY - 12, npmDisplay, { size: 14, weight: 700, fill: "#94a3b8", anchor: "end", mono: true })}
    <rect x="${chartX}" y="${chartY}" width="${chartWidth}" height="${barHeight}" rx="9" fill="#1e293b"/>
    <rect x="${chartX}" y="${chartY}" width="${npmWidth}" height="${barHeight}" rx="9" fill="#fb7185"/>

    ${text(chartX, chartY + gap - 12, "run-all-now", { size: 14, weight: 700, fill: "#e2e8f0" })}
    ${text(chartX + chartWidth, chartY + gap - 12, runDisplay, { size: 14, weight: 700, fill: "#94a3b8", anchor: "end", mono: true })}
    <rect x="${chartX}" y="${chartY + gap}" width="${chartWidth}" height="${barHeight}" rx="9" fill="#1e293b"/>
    <rect x="${chartX}" y="${chartY + gap}" width="${runWidth}" height="${barHeight}" rx="9" fill="#34d399"/>
  </g>`;
}

const speedSaved = timing.npmRunAll - timing.runAllNow;
const installSavedKiB = metrics.installed.npmRunAllNodeModulesKiB - metrics.installed.runAllNowNodeModulesKiB;
const installSavedPct = (installSavedKiB / metrics.installed.npmRunAllNodeModulesKiB) * 100;
const memorySaved = metrics.memory.npmRunAllPeakFootprintBytes - metrics.memory.runAllNowPeakFootprintBytes;
const memorySavedPct = (memorySaved / metrics.memory.npmRunAllPeakFootprintBytes) * 100;

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="900" viewBox="0 0 1200 900">
  <rect width="1200" height="900" fill="#05070d"/>
  ${pill(56, 44, "ZERO DEPENDENCIES")}
  ${pill(248, 44, "RUST CORE")}
  ${pill(372, 44, "API COMPATIBLE")}
  ${pill(536, 44, "MULTI-PLATFORM")}

  ${text(56, 128, "Drop in run-all-now.", { size: 50, weight: 900, fill: "#ffffff" })}
  ${text(56, 178, "Remove dependencies. Cut install size. Gain speed.", { size: 28, weight: 650, fill: "#cbd5e1" })}
  ${text(56, 218, `Measured on ${timing.platform || metrics.platform || "local"}: JS npm launcher → Rust binary`, { size: 17, weight: 500, fill: "#94a3b8" })}

  ${chart({
    x: 56,
    y: 270,
    title: "Dependencies",
    subtitle: "Direct runtime dependencies",
    npmLabel: "npm-run-all",
    npmValue: metrics.dependencies.npmRunAllDirectRuntimeDependencies,
    npmDisplay: String(metrics.dependencies.npmRunAllDirectRuntimeDependencies),
    runValue: Math.max(metrics.dependencies.runAllNowDirectRuntimeDependencies, 0.08),
    runDisplay: String(metrics.dependencies.runAllNowDirectRuntimeDependencies),
    delta: `${metrics.dependencies.npmRunAllDirectRuntimeDependencies} → 0 dependencies`,
    max: metrics.dependencies.npmRunAllDirectRuntimeDependencies
  })}

  ${chart({
    x: 602,
    y: 270,
    title: "Speed",
    subtitle: "Average wall-clock runtime",
    npmLabel: "npm-run-all",
    npmValue: timing.npmRunAll,
    npmDisplay: ms(timing.npmRunAll),
    runValue: timing.runAllNow,
    runDisplay: ms(timing.runAllNow),
    delta: `${ms(speedSaved)} saved · ${percent(timing.fasterPercent)} faster`,
    max: timing.npmRunAll
  })}

  ${chart({
    x: 56,
    y: 558,
    title: "Size",
    subtitle: "Installed node_modules footprint",
    npmLabel: "npm-run-all",
    npmValue: metrics.installed.npmRunAllNodeModulesKiB,
    npmDisplay: kib(metrics.installed.npmRunAllNodeModulesKiB),
    runValue: metrics.installed.runAllNowNodeModulesKiB,
    runDisplay: kib(metrics.installed.runAllNowNodeModulesKiB),
    delta: `${kib(installSavedKiB)} saved · ${percent(installSavedPct)} smaller`,
    max: metrics.installed.npmRunAllNodeModulesKiB
  })}

  ${chart({
    x: 602,
    y: 558,
    title: "Memory",
    subtitle: "Peak process footprint",
    npmLabel: "npm-run-all",
    npmValue: metrics.memory.npmRunAllPeakFootprintBytes,
    npmDisplay: bytes(metrics.memory.npmRunAllPeakFootprintBytes),
    runValue: metrics.memory.runAllNowPeakFootprintBytes,
    runDisplay: bytes(metrics.memory.runAllNowPeakFootprintBytes),
    delta: `${bytes(memorySaved)} saved · ${percent(memorySavedPct)} lower`,
    max: metrics.memory.npmRunAllPeakFootprintBytes
  })}
</svg>
`;

const svgPath = path.join(assetsDir, "comparison-chart.svg");
fs.writeFileSync(svgPath, svg);

console.log(`Wrote ${svgPath}`);

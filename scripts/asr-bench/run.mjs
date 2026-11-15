#!/usr/bin/env node
// BiliCatch 转写质量评测（离线、零依赖）
//
// 用法：
//   node run.mjs [cases.json]
//
// cases.json 结构：
//   {
//     "cases": [
//       {
//         "id": "talk-01",
//         "title": "口播：Rust 所有权",
//         "expectedEntities": ["所有权", "借用", "生命周期"],
//         "exports": { "aed": "out/talk-01.aed.txt", "baseline": "out/talk-01.base.txt" }
//       }
//     ]
//   }
//
// 说明：本脚本不运行 ASR，只对已导出的文本做客观统计，因此不会因缺模型而失败。
// 指标：实体召回率、疑似幻觉行数、空行率、与基线相比的实体增量。

import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));

// Whisper 在静音/音乐段常见的套话，用于标记疑似幻觉。
const HALLUCINATION_PHRASES = [
  "请不吝点赞",
  "订阅",
  "转发",
  "打赏",
  "明镜与点点",
  "字幕由",
  "谢谢观看",
  "Thank you for watching",
  "Subtitles by",
];

function readTranscript(path) {
  const absolute = resolve(HERE, path);
  if (!existsSync(absolute)) return null;
  const text = readFileSync(absolute, "utf8").replace(/^\uFEFF/, "");
  const lines = text.split(/\r?\n/).filter((line) => line.trim().length > 0);
  return { path: absolute, text, lines };
}

function analyze(transcript, expectedEntities) {
  const joined = transcript.text;
  const matched = expectedEntities.filter((entity) => joined.includes(entity));
  const hallucinationLines = transcript.lines.filter((line) =>
    HALLUCINATION_PHRASES.some((phrase) => line.includes(phrase)),
  );
  const characters = [...joined.replace(/\s/g, "")].length;
  return {
    characters,
    lines: transcript.lines.length,
    entityRecall: expectedEntities.length === 0 ? 1 : matched.length / expectedEntities.length,
    missingEntities: expectedEntities.filter((entity) => !joined.includes(entity)),
    hallucinationLines: hallucinationLines.length,
    charactersPerLine: transcript.lines.length === 0 ? 0 : Math.round(characters / transcript.lines.length),
  };
}

function loadCases(path) {
  const absolute = resolve(HERE, path);
  const raw = readFileSync(absolute, "utf8").replace(/^\uFEFF/, "");
  const document = JSON.parse(raw);
  if (!Array.isArray(document.cases) || document.cases.length === 0) {
    throw new Error(`${absolute} 里没有 cases 数组`);
  }
  return document.cases;
}

function percent(value) {
  return `${(value * 100).toFixed(1)}%`;
}

const casesPath = process.argv[2] ?? "cases.json";
let cases;
try {
  cases = loadCases(casesPath);
} catch (error) {
  console.error(`无法读取 ${casesPath}: ${error.message}`);
  console.error("请参考 cases.example.json 创建自己的评测集。");
  process.exit(2);
}

const rows = [];
const summary = {};

for (const testCase of cases) {
  const engines = Object.keys(testCase.exports ?? {});
  if (engines.length === 0) {
    console.warn(`跳过 ${testCase.id}：没有 exports`);
    continue;
  }
  for (const engine of engines) {
    const transcript = readTranscript(testCase.exports[engine]);
    if (!transcript) {
      rows.push({ case: testCase.id, engine, status: "缺失", entityRecall: "-", hallucination: "-", chars: "-" });
      continue;
    }
    const stats = analyze(transcript, testCase.expectedEntities ?? []);
    rows.push({ case: testCase.id, engine, status: "OK", stats });
    summary[testCase.id] ??= {};
    summary[testCase.id][engine] = stats;
  }
}

console.log("# BiliCatch 转写评测\n");
console.log("| 用例 | 引擎 | 实体召回 | 关键实体缺失 | 疑似幻觉行 | 字符数 | 平均行长 |");
console.log("| --- | --- | --- | --- | --- | --- | --- |");
for (const row of rows) {
  if (row.status !== "OK") {
    console.log(`| ${row.case} | ${row.engine} | 文件缺失 | - | - | - | - |`);
    continue;
  }
  const missing = row.stats.missingEntities.length === 0 ? "无" : row.stats.missingEntities.join("、");
  console.log(
    `| ${row.case} | ${row.engine} | ${percent(row.stats.entityRecall)} | ${missing} | ${row.stats.hallucinationLines} | ${row.stats.characters} | ${row.stats.charactersPerLine} |`,
  );
}

console.log("\n## 与基线的差值\n");
for (const [caseId, engines] of Object.entries(summary)) {
  const names = Object.keys(engines);
  if (names.length < 2) continue;
  const baseline = engines[names[0]];
  for (const name of names.slice(1)) {
    const candidate = engines[name];
    const recallDelta = percent(candidate.entityRecall - baseline.entityRecall);
    const hallucinationDelta = candidate.hallucinationLines - baseline.hallucinationLines;
    console.log(
      `- ${caseId}: ${name} 相对 ${names[0]} 实体召回 ${recallDelta}，幻觉行 ${hallucinationDelta >= 0 ? "+" : ""}${hallucinationDelta}`,
    );
  }
}

console.log("\n## 原始数据\n");
console.log("```json");
console.log(JSON.stringify(summary, null, 2));
console.log("```");

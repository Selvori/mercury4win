import { readFile, readdir } from "node:fs/promises";
import assert from "node:assert/strict";
const root = new URL("../../", import.meta.url);
const flatten = (object, prefix = "") => Object.entries(object).flatMap(([key, value]) => typeof value === "object" ? flatten(value, prefix + key + ".") : [[prefix + key, value]]);
const resources = await Promise.all(["en", "zh-Hans"].map(async language => {
  const text = await readFile(new URL(`src/i18n/${language}.json`, root), "utf8");
  assert(!text.startsWith("\ufeff"), "Unexpected BOM");
  return new Map(flatten(JSON.parse(text)));
}));
assert.deepEqual([...resources[0].keys()].sort(), [...resources[1].keys()].sort());
for (const [key, text] of resources[0]) {
  const parameters = value => [...value.matchAll(/\{\{([^}]+)\}\}/g)].map(match => match[1]).sort();
  assert.deepEqual(parameters(text), parameters(resources[1].get(key)), `Interpolation mismatch: ${key}`);
}
for (const directory of ["src/components/", "src/features/"]) {
  const url = new URL(directory, root);
  for (const file of await readdir(url, { recursive: true })) {
    if (!file.endsWith(".tsx")) continue;
    const text = await readFile(new URL(file.replaceAll("\\", "/"), url), "utf8");
    for (const match of text.matchAll(/\bt\(\s*["']([a-z][\w.]+)["']\s*[,)]/g)) {
      assert(resources[0].has(match[1]), `Missing static UI key ${match[1]} in ${directory}${file}`);
    }
  }
}
console.log(`PASS: ${resources[0].size} English / ${resources[1].size} Chinese keys; interpolation parameters, UTF-8 BOM and static UI key references match.`);

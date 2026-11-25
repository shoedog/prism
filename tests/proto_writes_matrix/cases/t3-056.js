function updateReport(path, value) {
  const keys = path.split(".");
  const report = { a: {} };
  let cursor = report;
  for (const key of keys.slice(0, -1)) {
    if (key === "__proto__" || key === "constructor" || key === "prototype") return report;
    cursor = cursor[key];
    if (cursor === null || typeof cursor !== "object") return report;
  }
  const last = keys[keys.length - 1];
  cursor[last] = value;
  return report;
}

module.exports = updateReport;

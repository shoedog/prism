function updateReport(path, value) {
  const keys = path.split(".");
  const report = { a: {} };
  let cursor = report;
  let pending = keys[0];
  for (const key of keys.slice(1)) {
    if (key === "__proto__" || key === "constructor" || key === "prototype") return report;
    cursor = cursor[pending];
    if (cursor === null || typeof cursor !== "object") return report;
    pending = key;
  }
  cursor[pending] = value;
  return report;
}

module.exports = updateReport;

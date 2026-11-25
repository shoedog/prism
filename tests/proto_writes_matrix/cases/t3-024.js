function updateReport(path, value) {
  const keys = path.split(".");
  const report = { a: {} };
  if (keys.some(key => key === "__proto__" || key === "constructor" || key === "prototype")) {
    return report;
  }
  let cursor = report;
  for (let index = 0; index < keys.length - 1; index++) {
    cursor = cursor[keys[index]];
    if (cursor === null || typeof cursor !== "object") return report;
  }
  cursor[keys[keys.length - 1]] = value;
  return report;
}

module.exports = updateReport;

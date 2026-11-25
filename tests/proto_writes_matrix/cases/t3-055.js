function updateReport(path, value) {
  const keys = path.split(".");
  const report = { a: {} };
  if (keys.some(key => key === "__proto__" || key === "constructor" || key === "prototype")) {
    return report;
  }
  const names = keys.map(key => key.replace(/-/g, ""));
  let cursor = report;
  for (let index = 0; index < names.length - 1; index++) {
    cursor = cursor[names[index]];
    if (cursor === null || typeof cursor !== "object") return report;
  }
  cursor[names[names.length - 1]] = value;
  return report;
}

module.exports = updateReport;

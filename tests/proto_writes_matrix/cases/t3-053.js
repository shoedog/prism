function updateReport(target, source) {
  for (const [key, incoming] of Object.entries(source)) {
    if (incoming !== null && typeof incoming === "object") {
      target[key] = updateReport(target[key] || { __proto__: null }, incoming);
    } else {
      target[key] = incoming;
    }
  }
  return target;
}

function buildReport(source) {
  const target = { a: { existing: 2 } };
  return updateReport(target, source);
}

module.exports = buildReport;

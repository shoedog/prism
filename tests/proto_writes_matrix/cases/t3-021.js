function updateReport(target, source) {
  for (const [key, incoming] of Object.entries(source)) {
    if (incoming !== null && typeof incoming === "object") {
      target[key] = updateReport({ __proto__: null, ...target[key] }, incoming);
    } else {
      target[key] = incoming;
    }
  }
  return target;
}

function buildReport(source) {
  const target = { __proto__: null, a: { existing: 2 } };
  return updateReport(target, source);
}

module.exports = buildReport;

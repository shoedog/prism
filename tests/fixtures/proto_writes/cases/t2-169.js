async function openPanels(names, field = "open") {
  const panels = { summary: {}, details: {} };
  const checks = names.map(async name => {
    if (name !== "summary" && name !== "details") {
      throw new RangeError("Unknown panel");
    }
    return name;
  });
  for (const result of await Promise.allSettled(checks)) {
    if (result.status === "fulfilled") panels[result.value][field] = true;
  }
  return panels;
}

module.exports = openPanels;

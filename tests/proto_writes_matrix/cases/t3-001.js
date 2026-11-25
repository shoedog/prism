function normalizeSections(sections) {
  for (const section of Object.keys(sections)) {
    const labels = sections[section];
    if (labels === null || typeof labels !== "object") continue;
    for (const key of Object.keys(labels)) {
      sections[section][key] = String(sections[section][key]).trim();
    }
  }
  return sections;
}

module.exports = normalizeSections;

function markSections(labels, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = { draft: {}, defaults };
  for (const label of Object.values(labels)) {
    if (typeof label !== "string" || !Object.hasOwn(sections, label)) continue;
    sections[label][field] = value;
  }
  return { previews, draft: sections.draft };
}

module.exports = markSections;

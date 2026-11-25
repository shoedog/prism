function markSection(section, field = "visible") {
  const sections = { draft: {}, final: {} };
  const normalize = text => text.trim().toLowerCase();
  if (typeof section !== "string") return sections;
  const checked = normalize(section);
  if (checked !== "draft" && checked !== "final") return sections;
  sections[normalize(section)][field] = true;
  return sections;
}

module.exports = markSection;

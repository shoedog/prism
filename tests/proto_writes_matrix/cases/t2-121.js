function annotateDraft(section, field, value) {
  const defaults = Object.create(null);
  const panels = { draft: {}, defaults };
  const selected = panels[section];
  if (section !== "draft") return panels.draft;
  selected[field] = value;
  return panels.draft;
}

function annotateDraftFromPath([section, field], value) {
  return annotateDraft(section, field, value);
}

module.exports = annotateDraftFromPath;

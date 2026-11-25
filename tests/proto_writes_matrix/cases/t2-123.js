async function loadReportSetting(selection, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  const section = await selection;
  if (sections[section] == null) return { previews, draft: sections.draft };
  sections[section][field] = value;
  return { previews, draft: sections.draft };
}

function loadFromPath([selection, field], value) {
  return loadReportSetting(Promise.resolve(selection), field, value);
}

module.exports = loadFromPath;

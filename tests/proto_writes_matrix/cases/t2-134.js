function saveAnnotation(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  let warning;
  try {
    if (section !== "draft") throw new RangeError("Expected a draft section");
  } catch (error) {
    warning = error.message;
  }
  if (sections[section] !== undefined) sections[section][field] = value;
  return { previews, warning, draft: sections.draft };
}

function saveFromPath([section, field], value) {
  return saveAnnotation(section, field, value);
}

module.exports = saveFromPath;

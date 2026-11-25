function updatePreviewLabel(section, field, value) {
  const defaults = Object.seal(
    Object.assign(Object.create(null), { label: "" })
  );
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  if (sections[section] !== undefined) sections[section][field] = value;
  return { previews, draft: sections.draft };
}

function updateFromPath([section, field], value) {
  return updatePreviewLabel(section, field, value);
}

module.exports = updateFromPath;

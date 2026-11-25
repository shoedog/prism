function firstPreview(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  function* rows() {
    try {
      yield previews[0];
      yield previews[1];
    } finally {
      if (sections[section] !== undefined) sections[section][field] = value;
    }
  }
  for (const row of rows()) return { row, previews, draft: sections.draft };
}

function previewFromPath([section, field], value) {
  return firstPreview(section, field, value);
}

module.exports = previewFromPath;

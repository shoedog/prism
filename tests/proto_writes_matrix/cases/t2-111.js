function annotatePreview(section, field, value) {
  const defaults = Object.freeze(Object.create(null));
  const previews = [Object.create(defaults), Object.create(defaults)];
  const catalog = Object.assign(Object.create(null), { draft: {}, defaults });
  if (catalog[section] != null) {
    catalog[section][field] = value;
  }
  return { previews, draft: catalog.draft };
}

module.exports = annotatePreview;

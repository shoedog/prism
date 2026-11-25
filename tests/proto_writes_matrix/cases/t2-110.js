function archiveSetting(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const draft = {};
  const catalog = Object.assign(Object.create(null), { draft, defaults });
  if (catalog[section] != null) {
    catalog[section][field] = (catalog[section] = {}, value);
  }
  return { previews, draft, replacement: catalog.draft };
}

module.exports = archiveSetting;

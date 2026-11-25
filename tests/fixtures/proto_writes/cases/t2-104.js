function setting(
  root,
  section,
  field,
  fallback,
  stored = (root[section][field] ??= fallback)
) {
  return stored;
}

function settingFromPath([section, field], fallback) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const root = Object.assign(Object.create(null), { draft: {}, defaults });
  const stored = setting(root, section, field, fallback);
  return { stored, previews, draft: root.draft };
}

module.exports = settingFromPath;

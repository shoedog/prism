function preview(root, section, field, value) {
  const draft = Object.create(null);
  if (field !== "label" && field !== "enabled") return draft;
  const base = root[section];
  if (base == null) return draft;
  Reflect.set(base, field, value, draft);
  return draft;
}

module.exports = preview;

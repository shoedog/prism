function annotateLocalPanel(section, field, value) {
  const profile = {};
  const display = {};
  const panels = { profile, display };
  const writable = new WeakSet([profile, display]);
  const target = panels[section];
  if (!writable.has(target)) return panels;
  target[field] = value;
  return panels;
}

module.exports = annotateLocalPanel;

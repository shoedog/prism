function annotateReportDefaults(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), {
    draft: Object.create(null), preview: previews[0]
  });
  const selected = sections[section];
  if (selected === undefined) return { previews };
  const base = Object.getPrototypeOf(selected);
  if (base !== null) base[field] = value;
  return { previews };
}

function annotateFromPath([section, field], value) {
  return annotateReportDefaults(section, field, value);
}

module.exports = annotateFromPath;

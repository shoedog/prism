function annotateVisibleSection(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const { archive, ...visible } = {
    draft: Object.create(null), defaults, archive: Object.create(null)
  };
  if (Object.hasOwn(visible, section)) visible[section][field] = value;
  return { archive, previews, draft: visible.draft };
}

function annotateFromPath([section, field], value) {
  return annotateVisibleSection(section, field, value);
}

module.exports = annotateFromPath;

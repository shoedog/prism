function updateSections(keys, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  let warnings = 0;
  for (const section of keys) {
    switch (section) {
      case "defaults":
        warnings++;
        break;
    }
    if (sections[section] !== undefined) sections[section][field] = value;
  }
  return { previews, warnings, draft: sections.draft };
}

function fromPath(path, value) {
  return updateSections(path.slice(0, -1), path[path.length - 1], value);
}

module.exports = fromPath;

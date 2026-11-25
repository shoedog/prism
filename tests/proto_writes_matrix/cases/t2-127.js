function resetSection(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  [
    sections[section],
    sections[section][field]
  ] = [Object.create(null), value];
  return { sections, previews };
}

function resetSectionFromPath([section, field], value) {
  return resetSection(section, field, value);
}

module.exports = resetSectionFromPath;

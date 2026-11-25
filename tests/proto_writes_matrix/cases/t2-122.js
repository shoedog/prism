function createReportEntry(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = { draft: {}, defaults };
  if (!Object.hasOwn(sections, section)) return { previews, draft: sections.draft };
  class Entry {
    stored = (sections[section][field] = value);
  }
  return { entry: new Entry(), previews, draft: sections.draft };
}

function createFromPath([section, field], value) {
  return createReportEntry(section, field, value);
}

module.exports = createFromPath;

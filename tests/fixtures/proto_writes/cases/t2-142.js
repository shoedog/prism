function createReportType(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  if (sections[section] === undefined) return { previews, draft: sections.draft };
  class Report {
    [(sections[section][field] = value, "summary")]() {
      return "Report";
    }
  }
  return { Report, previews, draft: sections.draft };
}

function createFromPath([section, field], value) {
  return createReportType(section, field, value);
}

module.exports = createFromPath;

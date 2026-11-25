function annotateParsedSection(path, separator, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  if (path !== "draft") return { previews, draft: sections.draft };
  const [section] = path.split(separator);
  if (sections[section] !== undefined) sections[section][field] = value;
  return { previews, draft: sections.draft };
}

function annotateFromPath([section, field], value) {
  const separator = { [Symbol.split]: () => [section] };
  return annotateParsedSection("draft", separator, field, value);
}

module.exports = annotateFromPath;

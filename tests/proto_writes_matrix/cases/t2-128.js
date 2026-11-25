function annotateSection(section, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  const receiver = sections[section];
  const schema = Object.create(null);
  if (receiver !== undefined) Reflect.set(schema, field, value, receiver);
  return { previews, draft: sections.draft };
}

function annotateFromPath([section, field], value) {
  return annotateSection(section, field, value);
}

module.exports = annotateFromPath;

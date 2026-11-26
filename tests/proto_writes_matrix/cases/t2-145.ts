function annotateDraft(section: string, field: string, value: unknown) {
  function isDraftSection(key: string): key is "draft" {
    return key.length > 0;
  }
  const defaults: Record<string, unknown> = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections: Record<string, Record<string, unknown> | undefined> =
    Object.assign(Object.create(null), { draft: Object.create(null), defaults });
  if (isDraftSection(section) && sections[section] !== undefined) {
    sections[section][field] = value;
  }
  return { previews, draft: sections.draft };
}

function annotateFromPath(path: string[], value: unknown) {
  const [section, field] = path;
  return annotateDraft(section, field, value);
}

module.exports = annotateFromPath;

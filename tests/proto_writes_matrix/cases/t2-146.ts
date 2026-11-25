function annotateDraft(section: unknown, field: string, value: unknown) {
  function assertDraft(key: unknown): asserts key is "draft" {
    if (key !== "draft") throw new RangeError("Expected a draft section");
  }
  const defaults: Record<string, unknown> = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), {
    draft: Object.create(null), defaults
  });
  assertDraft(section);
  sections[section][field] = value;
  return { draft: sections.draft, previews };
}

function annotateDraftFromPath([section, field]: [string, string], value: unknown) {
  return annotateDraft(section, field, value);
}

module.exports = annotateDraftFromPath;

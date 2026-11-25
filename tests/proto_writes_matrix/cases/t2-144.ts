function stageReportLabel(section: string, field: string, value: unknown) {
  const defaults: Record<string, unknown> = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections: Record<string, Record<string, unknown> | undefined> =
    Object.assign(Object.create(null), { draft: Object.create(null), defaults });
  using session = {
    previews,
    draft: sections.draft,
    [Symbol.dispose]() {
      const selected = sections[section];
      if (selected !== undefined) selected[field] = value;
    }
  };
  return { previews: session.previews, draft: session.draft };
}

function stageFromPath(path: string[], value: unknown) {
  const [section, field] = path;
  return stageReportLabel(section, field, value);
}

module.exports = stageFromPath;

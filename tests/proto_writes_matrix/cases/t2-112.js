function defineReportFields(section, titleSpec, summarySpec) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const catalog = Object.assign(Object.create(null), { draft: {}, defaults });
  const target = catalog[section];
  if (target == null) return { previews, applied: false };
  try {
    Object.defineProperties(target, { title: titleSpec, summary: summarySpec });
    return { previews, draft: catalog.draft, applied: true };
  } catch (error) {
    if (!(error instanceof TypeError)) throw error;
    return { previews, applied: false };
  }
}

module.exports = defineReportFields;

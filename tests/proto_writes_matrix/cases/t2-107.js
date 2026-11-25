function patchReports(source) {
  const defaults = Object.create(null);
  const reports = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  function apply(patch) {
    for (const [key, value] of Object.entries(patch)) {
      const child = Object.hasOwn(this, key) ? this[key] : undefined;
      if (child && typeof child === "object" &&
          value && typeof value === "object") {
        apply.call(child, value);
      } else {
        this[key] = value;
      }
    }
  }
  apply.call(sections, source);
  return { reports, draft: sections.draft };
}

module.exports = patchReports;

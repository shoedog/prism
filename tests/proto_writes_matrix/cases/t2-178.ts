function recordSection(section: "draft" | "final", field: "label" = "label") {
  const pages = { draft: { label: "" }, final: { label: "" } };
  const active = "draft";
  const page: (typeof pages)[typeof section] = pages[active];
  page[field] = section;
  return pages;
}

module.exports = recordSection;

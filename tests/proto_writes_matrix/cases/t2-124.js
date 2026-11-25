function annotateExport(namespace, section, field, value) {
  if (typeof section !== "string") return false;
  const bucket = namespace[section];
  if (bucket == null || typeof bucket !== "object") return false;
  bucket[field] = value;
  return true;
}

function annotateExportFromPath([section, field], value) {
  const namespace = Object.freeze(Object.assign(Object.create(null), {
    draft: Object.create(null)
  }));
  const annotated = annotateExport(namespace, section, field, value);
  return { annotated, draft: namespace.draft };
}

module.exports = annotateExportFromPath;

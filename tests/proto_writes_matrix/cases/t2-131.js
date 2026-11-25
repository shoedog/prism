function annotatePath(keys, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), {
    draft: Object.create(null), undefined: defaults
  });
  if (!keys.every(key => key === "draft")) return { previews, draft: sections.draft };
  let cursor = sections;
  for (const key of keys) {
    cursor = cursor[key];
    if (cursor === undefined) return { previews, draft: sections.draft };
  }
  cursor[field] = value;
  return { previews, draft: sections.draft };
}

function annotateFromPath(path, value) {
  const keys = [];
  for (let i = 0; i < path.length - 1; i++) {
    if (path[i] !== "") keys[i] = path[i];
  }
  keys.length = path.length - 1;
  return annotatePath(keys, path[path.length - 1], value);
}

module.exports = annotateFromPath;

function annotateSections(keys, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  const excluded = /^defaults$/g;
  for (const key of keys) {
    if (excluded.test(key)) continue;
    if (sections[key] !== undefined) sections[key][field] = value;
  }
  return { previews, draft: sections.draft };
}

function fromPath(path, value) {
  return annotateSections(path.slice(0, -1), path[path.length - 1], value);
}

module.exports = fromPath;

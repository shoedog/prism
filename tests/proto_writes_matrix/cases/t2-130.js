function previewAnnotation(section, field, value) {
  const ignored = [];
  const defaults = new Proxy(Object.create(null), {
    set(_target, key) {
      ignored.push(key);
      return true;
    }
  });
  const previews = [Object.create(defaults), Object.create(defaults)];
  const sections = Object.assign(Object.create(null), { draft: {}, defaults });
  if (sections[section] !== undefined) sections[section][field] = value;
  return { previews, ignored, draft: sections.draft };
}

function previewAnnotationFromPath([section, field], value) {
  return previewAnnotation(section, field, value);
}

module.exports = previewAnnotationFromPath;

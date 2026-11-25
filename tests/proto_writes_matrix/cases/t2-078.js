function createSection(section, field, value) {
  const root = {};
  const key = String(section);
  Object.defineProperty(root, key, {
    value: Object.create(null),
    enumerable: true
  });
  root[key][field] = value;
  return root;
}

function createSectionAtPath(path, value) {
  const [section, field] = path;
  return createSection(section, field, value);
}

module.exports = createSectionAtPath;

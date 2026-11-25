function build(path, field, value) {
  const root = Object.create(null);
  let cursor = root;
  for (const key of path.split(".")) {
    if (cursor[key] == null) cursor[key] = Object.create(null);
    cursor = cursor[key];
  }
  cursor[field] = value;
  return root;
}

module.exports = build;

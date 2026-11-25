function previewLabel(req, value) {
  const { section, field } = req.params;
  const labels = { a: {} };
  let cursor = labels[section];
  const existed = cursor !== undefined;
  cursor = {};
  cursor[field] = value;
  return { existed, cursor };
}

function previewAtPath(path, value) {
  return previewLabel({ params: { section: path[0], field: path[1] } }, value);
}

module.exports = previewAtPath;

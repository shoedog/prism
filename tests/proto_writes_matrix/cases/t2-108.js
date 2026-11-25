function recordVisit(root, keys, field, value) {
  let cursor = root;
  let visited = 0;
  for (const key of keys) {
    if (cursor == null) break;
    cursor = cursor[key];
    visited++;
  }
  cursor = {};
  cursor[field] = value;
  return { visited, fields: cursor };
}

function recordVisitAtPath(root, path, value) {
  return recordVisit(root, path.slice(0, -1), path.at(-1), value);
}

module.exports = recordVisitAtPath;

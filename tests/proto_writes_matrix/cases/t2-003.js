function set(root, parts, value) {
  const keys = parts.slice();
  const last = keys.shift();
  const parent = keys.reduceRight((cursor, key) => {
    return cursor[key];
  }, root);
  parent[last] = value;
  return root;
}

module.exports = set;

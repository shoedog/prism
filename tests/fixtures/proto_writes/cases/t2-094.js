function loadCounts(root, text, field) {
  JSON.parse(text, (section, count) => {
    if (section && typeof count === "number" && root[section] != null) {
      root[section][field] = count;
    }
    return count;
  });
  return root;
}

function loadFromParts(root, parts, count) {
  const keys = parts.slice(0, -1);
  const field = parts[parts.length - 1];
  const text = JSON.stringify(Object.fromEntries(keys.map(key => [key, count])));
  return loadCounts(root, text, field);
}

module.exports = loadFromParts;

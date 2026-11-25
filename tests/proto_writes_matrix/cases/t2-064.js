function set(root, input, property, value) {
  const cursor = Object.entries(input).reduce((current, entry) => {
    const [key, enabled] = entry;
    return enabled ? current[key] : current;
  }, root);
  if (cursor != null) {
    cursor[property] = value;
  }
  return root;
}

function setFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  const input = Object.fromEntries(keys.map(key => [key, true]));
  return set(root, input, property, value);
}

module.exports = setFromParts;

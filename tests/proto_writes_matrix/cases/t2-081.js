function update(root, keys, field, value) {
  let recognized = 0;
  keys.forEach(key => {
    if (key !== "profile" && key !== "settings") return;
    recognized++;
  });
  let cursor = root;
  for (const key of keys) {
    cursor = cursor[key];
    if (cursor == null) return recognized;
  }
  cursor[field] = value;
  return recognized;
}

function updateFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const field = parts[parts.length - 1];
  update(root, keys, field, value);
  return root;
}

module.exports = updateFromParts;

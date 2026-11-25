function put(root, keys, field, value) {
  const result = keys.reduce((state, key) => {
    if (state.cursor == null) return state;
    return { cursor: state.cursor[key], depth: state.depth + 1 };
  }, { cursor: root, depth: 0 });
  if (result.cursor != null) result.cursor[field] = value;
  return result.depth;
}

function putFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const field = parts[parts.length - 1];
  put(root, keys, field, value);
  return root;
}

module.exports = putFromParts;

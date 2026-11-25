function descend(keys, root, value) {
  if (keys.length === 1) {
    root[keys[0]] = value;
    return root;
  }
  const [key, ...rest] = keys;
  descend(rest, root[key], value);
  return root;
}

module.exports = descend;

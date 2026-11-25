function put(root, text, value) {
  const match = /^([^/]+)\/([^/]+)$/.exec(text);
  if (!match) return root;
  const [, selector, property] = match;
  root[selector][property] = value;
  return root;
}

module.exports = put;

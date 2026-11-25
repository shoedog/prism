function put(root, text, property, value) {
  const match = /^(?<section>[^:]+):/.exec(text);
  if (!match) return root;
  const selector = match.groups.section;
  if (root[selector] == null) return root;
  root[selector][property] = value;
  return root;
}

function putFromText(root, text, value) {
  const colon = text.indexOf(":");
  if (colon < 0) return root;
  return put(root, text, text.slice(colon + 1), value);
}

module.exports = putFromText;

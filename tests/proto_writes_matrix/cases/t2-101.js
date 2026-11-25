async function put(root, sections, field, value) {
  let cursor = root;
  for await (const section of sections) {
    cursor = cursor[section];
    if (cursor == null) return root;
  }
  cursor[field] = value;
  return root;
}

async function* iterate(items) {
  for (const item of items) yield item;
}

function putFromParts(root, parts, value) {
  const sections = iterate(parts.slice(0, -1));
  const field = parts[parts.length - 1];
  return put(root, sections, field, value);
}

module.exports = putFromParts;

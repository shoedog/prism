function* set(root, keys, property, value) {
  let cursor = root;
  for (const key of keys) {
    cursor = cursor[key];
    yield cursor;
  }
  if (cursor != null) cursor[property] = value;
  return root;
}

function setFromParts(root, parts, value) {
  const keys = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  const iterator = set(root, keys, property, value);
  let step = iterator.next();
  while (!step.done) step = iterator.next();
  return step.value;
}

module.exports = setFromParts;

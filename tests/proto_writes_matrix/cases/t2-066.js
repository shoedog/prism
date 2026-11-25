function set(root, paths, property, value) {
  let cursor = root;
  outer: for (const path of paths) {
    for (const key of path.split(".")) {
      cursor = cursor[key];
      if (cursor == null) return root;
      continue outer;
    }
  }
  cursor[property] = value;
  return root;
}

function setFromParts(root, parts, value) {
  const paths = parts.slice(0, -1);
  const property = parts[parts.length - 1];
  return set(root, paths, property, value);
}

module.exports = setFromParts;

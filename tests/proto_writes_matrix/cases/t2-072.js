function set(root, pointer, property, value) {
  let cursor = root;
  for (const raw of pointer.split("/").slice(1)) {
    if (raw === "__proto__" || raw === "constructor") return root;
    const key = raw.replace(/~1/g, "/").replace(/~0/g, "~");
    cursor = cursor[key];
    if (cursor == null) return root;
  }
  cursor[property] = value;
  return root;
}

function setFromPointer(root, path, value) {
  const tokens = path.split("/");
  const property = tokens.pop();
  return set(root, tokens.join("/"), property, value);
}

module.exports = setFromPointer;

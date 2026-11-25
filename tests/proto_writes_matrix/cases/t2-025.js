function set(root, path, value) {
  const keys = path.split(".");
  const last = keys.pop();
  let cursor = root;
  for (const key of keys) {
    switch (key) {
      case "__proto__": case "constructor": case "prototype": return;
    }
    cursor = cursor[key];
  }
  cursor[last] = value;
  return root;
}

module.exports = set;

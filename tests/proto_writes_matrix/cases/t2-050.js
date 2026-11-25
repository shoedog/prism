function set(root, path, value) {
  if (path.startsWith("__proto__")) return root;
  const dot = path.indexOf(".");
  if (dot < 0) {
    root[path] = value;
  } else {
    const key = path.slice(0, dot);
    set(root[key], path.slice(dot + 1), value);
  }
  return root;
}

module.exports = set;

function populate(root, input) {
  for (const [path, value] of Object.entries(input)) {
    const [selector, property] = path.split(".");
    if (property === undefined) continue;
    root[selector][property] = value;
  }
  return root;
}

module.exports = populate;

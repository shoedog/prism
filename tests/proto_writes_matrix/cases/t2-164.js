function countLeafNames(tree) {
  const counts = Object.create(null);
  function visit(node, out) {
    for (const key of Object.keys(node)) {
      const value = node[key];
      if (value !== null && typeof value === "object") {
        visit(node[key], out);
      } else {
        out[key] = (out[key] ?? 0) + 1;
      }
    }
  }
  visit(tree, counts);
  return counts;
}

module.exports = countLeafNames;

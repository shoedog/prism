function drawLabels(path, labels) {
  const [level] = path;
  const grid = [[null, null], [null, null]];
  const nodes = [{ level: 0, lane: 0 }, { level: 0, lane: 1 }];
  nodes[1].level = level;
  for (const node of nodes) {
    grid[node.level][node.lane] = labels[node.lane];
  }
  return grid;
}

module.exports = drawLabels;

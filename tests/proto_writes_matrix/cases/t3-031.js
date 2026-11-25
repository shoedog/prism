function drawLabels(path, labels) {
  const grid = [[null, null], [null, null]];
  const nodes = labels.map((label, lane) => ({ level: path[lane], lane }));
  for (const node of nodes) {
    grid[node.level][node.lane] = labels[node.lane];
  }
  return grid;
}

module.exports = drawLabels;

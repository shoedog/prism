function drawLabels(labels) {
  const grid = [[null, null], [null, null]];
  const nodes = [{ level: 0, lane: 0 }, { level: 0, lane: 1 }];
  for (const node of nodes) {
    node.level += 1;
    grid[node.level][node.lane] = labels[node.lane];
  }
  return grid;
}

module.exports = drawLabels;

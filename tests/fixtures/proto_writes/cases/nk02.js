function fill(grid, bounds) {
  const [n, m] = bounds.map(Number);
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < m; c++) {
      grid[r][c] = 0;
    }
  }
  return grid;
}

module.exports = fill;

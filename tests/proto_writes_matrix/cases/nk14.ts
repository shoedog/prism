type Cell = number;

function put(grid: Cell[][], y: number, x: number, v: Cell) {
  grid[y][x] = v;
}

function putFromJson(grid: Cell[][], coordinates: string[], v: Cell) {
  const text = JSON.stringify(coordinates);
  const [y, x] = JSON.parse(text);
  put(grid, y, x, v);
  return grid;
}

module.exports = putFromJson;

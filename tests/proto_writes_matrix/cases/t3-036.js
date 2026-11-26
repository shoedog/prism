function moveSegment(path, delta) {
  const [startIdx, horizontal] = path;
  const points = [[0, 0], [1, 0], [2, 0]];
  const axis = horizontal ? 0 : 1;
  points[startIdx][axis] = delta;
  return points;
}

module.exports = moveSegment;

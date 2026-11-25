function copy(m, rows) {
  var i, j;
  for (i = 0; i < rows.length; i++) {
    for (j = 0; j < rows[i].length; j++) {
      m[i][j] = rows[i][j];
    }
  }
  return m;
}

module.exports = copy;

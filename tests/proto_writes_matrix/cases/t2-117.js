function markFirstToken(rows, text, field, value) {
  const match = /\S/.exec(text);
  if (match == null || match.index >= rows.length) return rows;
  const index = match.index;
  rows[index][field] = value;
  return rows;
}

module.exports = markFirstToken;

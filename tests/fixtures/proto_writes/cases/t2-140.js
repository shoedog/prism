function annotateRow(index, field, value) {
  const defaults = Object.create(null);
  const previews = [Object.create(defaults), Object.create(defaults)];
  const fallbackRows = { 1: defaults };
  const rows = [{}, , {}];
  Object.setPrototypeOf(rows, fallbackRows);
  if (!Number.isInteger(index) || index < 0 || index >= rows.length) return { previews, rows };
  rows[index][field] = value;
  return { previews, rows };
}

function annotateFromPath([index, field], value) {
  return annotateRow(Number(index), field, value);
}

module.exports = annotateFromPath;

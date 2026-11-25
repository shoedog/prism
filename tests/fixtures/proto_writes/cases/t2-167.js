function setColumnLabel(path, value) {
  const sheet = { labels: Object.create(null) };
  const parts = path.split(".");
  if (parts.length !== 2 || parts[0] !== "labels") return sheet;
  const field = parts.pop();
  let cursor = sheet;
  for (const key of parts) cursor = cursor[key];
  cursor[field] = value;
  return sheet;
}

module.exports = setColumnLabel;

function groupField(path, value, templates) {
  const [group, field] = path;
  let table;
  table = Object.create(null);
  if (!(group in table)) table[group] = templates[group] || {};
  table[group][field] = value;
  return table;
}

module.exports = groupField;

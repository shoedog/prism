function groupField(path, value) {
  const [group, field] = path;
  let table;
  table = Object.create(null);
  if (!(group in table)) table[group] = {};
  table[group][field] = value;
  return table;
}

module.exports = groupField;

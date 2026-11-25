function groupItem(path, value) {
  const [group, index] = path;
  let table;
  table = Object.create(null);
  if (!(group in table)) table[group] = [];
  table[group][index] = value;
  return table;
}

module.exports = groupItem;

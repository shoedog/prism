function annotateGroup(rows, section, field, value) {
  const groups = Object.groupBy(rows, row => row.section);
  if (groups[section] == null) return groups;
  groups[section][field] = value;
  return groups;
}

module.exports = annotateGroup;

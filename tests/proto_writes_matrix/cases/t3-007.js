function groupCount(path, value) {
  const [category, status] = path;
  const groups = { "a:b": {} };
  const group = `${category}:${status}`;
  if (!groups[group]) groups[group] = {};
  groups[group].count = value;
  return groups;
}

module.exports = groupCount;

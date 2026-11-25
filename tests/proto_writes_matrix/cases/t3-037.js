function addScoped(path, value) {
  const [section, name] = path;
  const groups = { a_b: {} };
  const group = `${section}_${name}`;
  if (!groups[group]) groups[group] = {};
  groups[group].count = value;
  return groups;
}

module.exports = addScoped;

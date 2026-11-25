function labelStatus(name, field) {
  const statuses = ["open", "closed"];
  const rows = [{}, {}];
  const position = statuses.indexOf(name);
  if (position < 0) return rows;
  rows[position][field] = name;
  return rows;
}

module.exports = labelStatus;

function labelStatus(name, field) {
  const statuses = {
    indexOf(value) {
      return value;
    }
  };
  const rows = { open: {}, closed: {} };
  const position = statuses.indexOf(name);
  rows[position][field] = name;
  return rows;
}

module.exports = labelStatus;

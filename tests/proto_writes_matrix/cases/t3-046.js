function rejectField() {
  throw new RangeError("Unknown field");
}

function updateLabel(path, value) {
  const [section, field] = path.split(".");
  const labels = { a: {} };
  if (field === "__proto__" || field === "constructor" || field === "prototype") rejectField();
  labels[section][field] = value;
  return labels;
}

module.exports = updateLabel;

function missingSection() {
  throw new RangeError("Unknown section");
}

function updateLabel(path, value) {
  const [section, field] = path.split(".");
  const labels = { a: {} };
  if (!Object.hasOwn(labels, section)) missingSection();
  labels[section][field] = value;
  return labels;
}

module.exports = updateLabel;

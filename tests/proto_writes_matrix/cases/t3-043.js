function enableLabel(path, value) {
  const [section] = path.split(".");
  const labels = { a: {} };
  labels[section].enabled = value;
  return labels;
}

module.exports = enableLabel;

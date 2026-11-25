function save(root, section, field, value) {
  try {
    if (section !== "settings" && section !== "profile") return false;
    return true;
  } finally {
    if (root[section] != null) {
      root[section][field] = value;
    }
  }
}

function saveFromParts(root, parts, value) {
  const [section, field] = parts;
  save(root, section, field, value);
  return root;
}

module.exports = saveFromParts;

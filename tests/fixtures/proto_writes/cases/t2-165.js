function countEnabled(paths, field = "matches") {
  const root = { shop: { open: true }, team: { active: true } };
  let cursor = root;
  let matches = 0;
  for (const path of paths) {
    const start = cursor;
    try {
      for (const key of path.split(".")) {
        if (cursor == null) break;
        cursor = cursor[key];
      }
      if (cursor === true) matches++;
    } finally {
      cursor = start;
    }
  }
  cursor[field] = matches;
  return root;
}

module.exports = countEnabled;

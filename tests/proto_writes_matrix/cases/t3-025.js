function copySections(source, target) {
  for (const section of Object.keys(source)) {
    const labels = source[section];
    if (labels === null || typeof labels !== "object") continue;
    if (!target[section]) target[section] = {};
    for (const key of Object.keys(labels)) {
      target[section][key] = String(source[section][key]).trim();
    }
  }
  return target;
}

module.exports = copySections;

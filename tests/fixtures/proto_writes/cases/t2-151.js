function annotateSection(path, field, value) {
  const schema = { account: { theme: {} }, team: { roster: {} } };
  const data = { account: { theme: {} }, team: { roster: {} } };
  let rule = schema;
  let cursor = data;
  for (const key of path.split(".")) {
    if (!Object.hasOwn(rule, key)) return data;
    rule = rule[key];
    cursor = cursor[key];
  }
  cursor[field] = value;
  return data;
}

module.exports = annotateSection;

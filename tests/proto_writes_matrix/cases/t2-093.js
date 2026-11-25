function put(root, token, field, value) {
  const sections = new Map([
    ["account", "profile"],
    ["view", "settings"]
  ]);
  const section = sections.get(token);
  if (section === undefined) return root;
  root[section][field] = value;
  return root;
}

module.exports = put;

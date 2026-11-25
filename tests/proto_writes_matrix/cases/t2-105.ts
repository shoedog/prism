type Sections = { settings: Record<string, unknown> };

function put(section: keyof Sections, field: string, value: unknown) {
  const root: Sections = { settings: {} };
  root[section][field] = value;
  return root;
}

function putFromPath(path: string[], value: unknown) {
  const [section, field] = path as [keyof Sections, string];
  return put(section, field, value);
}

module.exports = putFromPath;

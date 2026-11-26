function put(root, [key, field], value) {
  const table = { __proto__: null, a: {} };
  if (typeof key !== "string") return table;
  if (!Object.hasOwn(table, key)) table[key] = {};
  table[key][field] = value;
  return table;
}

module.exports = put;

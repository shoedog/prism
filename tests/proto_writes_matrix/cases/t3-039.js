function extendType(path, value) {
  const [type, name] = path;
  let types;
  types = Object.create(null);
  types.object = Object.prototype;
  types.array = Array.prototype;
  if (!(type in types)) types[type] = {};
  types[type][name] = value;
  return types;
}

module.exports = extendType;

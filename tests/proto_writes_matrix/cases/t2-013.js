function put(path, value) {
  const root = Object.create(null);
  root.records = { a: {} };
  const [first, second, property] = path;
  root[first][second][property] = value;
  return root;
}

module.exports = put;

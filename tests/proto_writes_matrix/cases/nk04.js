function put(store, kind, id, v) {
  store[kind][id] = v;
  return store;
}

module.exports = put;

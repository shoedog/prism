function put(cache, id, f, v) {
  cache[`user:${id}`][f] = v;
  return cache;
}

module.exports = put;

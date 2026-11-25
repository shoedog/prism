function addBucket(path, value) {
  const [name, field] = path.split(".");
  const buckets = {};
  Object.defineProperty(buckets, name, { value: {}, enumerable: true });
  buckets[name][field] = value;
  return buckets;
}

module.exports = addBucket;

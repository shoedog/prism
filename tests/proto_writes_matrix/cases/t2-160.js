function recordMatch(text, field = "last") {
  const buckets = { true: {}, false: {} };
  const matched = /\d/.test(text);
  buckets[matched][field] = text;
  return buckets;
}

module.exports = recordMatch;

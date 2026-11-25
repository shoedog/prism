function enable(obj, path) {
  let o = obj;
  for (const s of path.split(".")) {
    o = o[s];
  }
  o.enabled = true;
  return obj;
}

module.exports = enable;

const KEY = "enabled";

function set(obj, path, v) {
  let o = obj;
  for (const s of path.split(".")) {
    o = o[s];
  }
  o[KEY] = v;
  return obj;
}

module.exports = set;

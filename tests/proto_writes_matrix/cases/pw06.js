function setPointer(obj, pointer, v) {
  const keys = pointer.split("/").slice(1);
  const last = keys.pop();
  let o = obj;
  for (const k of keys) {
    o = o[k] = o[k] || {};
  }
  o[last] = v;
}

module.exports = setPointer;

function set(obj, path, v) {
  const keys = path.split(".");
  const last = keys.pop();
  const st = { cur: obj };
  for (const k of keys) {
    st.cur = st.cur[k];
  }
  st.cur[last] = v;
  return obj;
}

module.exports = set;

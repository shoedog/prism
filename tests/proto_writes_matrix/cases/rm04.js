function copy(t, s) {
  for (const k in s) t[k] = s[k];
}

module.exports = copy;

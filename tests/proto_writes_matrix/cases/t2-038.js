function set(root, keys, value) {
  function even(cursor, i) {
    if (i === keys.length - 1) {
      cursor[keys[i]] = value;
      return;
    }
    odd(cursor[keys[i]], i + 1);
  }
  function odd(cursor, i) {
    even(cursor[keys[i]], i + 1);
  }
  if (keys.length) even(root, 0);
  return root;
}

module.exports = set;

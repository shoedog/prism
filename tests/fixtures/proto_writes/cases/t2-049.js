function set(root, key, property, value, checked) {
  let selector;
  if (checked) {
    if (key === "__proto__" || key === "constructor") return root;
    selector = key;
  } else {
    selector = key;
  }
  root[selector][property] = value;
  return root;
}

function putFromParts(root, [key, property, branch], value) {
  return set(root, key, property, value, branch === "true");
}

module.exports = putFromParts;

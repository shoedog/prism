function put(root, usePrototype, property, value) {
  const selector = usePrototype ? "__proto__" : "records";
  if (root[selector] == null) return root;
  root[selector][property] = value;
  return root;
}

function putFromParts(root, [branch, property], value) {
  return put(root, branch === "true", property, value);
}

module.exports = putFromParts;

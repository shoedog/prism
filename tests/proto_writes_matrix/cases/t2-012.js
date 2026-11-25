function put(root, value) {
  const selector = "records";
  const property = "enabled";
  root[selector][property] = value;
  return root;
}

module.exports = put;

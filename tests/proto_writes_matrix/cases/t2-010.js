function put(root, selector, value) {
  const outer = "records";
  const property = "enabled";
  root[outer][selector][property] = value;
  return root;
}

module.exports = put;

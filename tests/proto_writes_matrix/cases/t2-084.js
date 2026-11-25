function put(root, section, field, value) {
  const slot = Object.getOwnPropertyDescriptor(root, section);
  if (!slot || !Object.hasOwn(slot, "value") || slot.value == null) {
    return root;
  }
  slot.value[field] = value;
  return root;
}

module.exports = put;

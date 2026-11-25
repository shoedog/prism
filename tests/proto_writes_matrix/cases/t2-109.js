function updateSnapshot(root, keys, field, value) {
  function copyBranch(node, depth) {
    const copy = { ...node };
    if (depth === keys.length) {
      copy[field] = value;
      return copy;
    }
    const key = keys[depth];
    const child = Object.hasOwn(copy, key) ? copy[key] : undefined;
    return { ...copy, [key]: copyBranch(child, depth + 1) };
  }
  return copyBranch(root, 0);
}

function updateSnapshotAtPath(root, path, value) {
  return updateSnapshot(root, path.slice(0, -1), path.at(-1), value);
}

module.exports = updateSnapshotAtPath;

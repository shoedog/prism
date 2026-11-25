async function updateWhenReady(root, keys, field, value, ready) {
  const allowed = new Set(["settings", "profile"]);
  if (!keys.every(key => allowed.has(key))) return root;
  await ready;
  let cursor = root;
  for (const key of keys) {
    cursor = cursor[key];
    if (cursor == null) return root;
  }
  cursor[field] = value;
  return root;
}

function updateFromParts(root, parts, value) {
  const replacement = parts.slice(0, -1);
  const field = parts[parts.length - 1];
  const keys = ["settings"];
  let release;
  const ready = new Promise(resolve => {
    release = resolve;
  });
  const pending = updateWhenReady(root, keys, field, value, ready);
  keys.splice(0, keys.length, ...replacement);
  release();
  return pending;
}

module.exports = updateFromParts;

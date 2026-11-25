function markSnapshot(section, field = "marked") {
  const defaults = { marked: false };
  const live = { defaults, mirror: defaults, card: Object.create(defaults) };
  const snapshot = structuredClone(live);
  if (typeof section === "string" && Object.hasOwn(snapshot, section)) {
    snapshot[section][field] = true;
  }
  return { live, snapshot };
}

module.exports = markSnapshot;

function recordParts(text, field = "text") {
  const parts = { area: {}, item: {} };
  const match = /^(?<area>[^/]+)\/(?<item>.+)$/.exec(text);
  if (!match) return parts;
  for (const [key, captured] of Object.entries(match.groups)) {
    parts[key][field] = captured;
  }
  return parts;
}

module.exports = recordParts;

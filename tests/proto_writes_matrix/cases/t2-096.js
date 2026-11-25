function put(text, section, field, value) {
  if (typeof text !== "string") return text;
  if (text[section] == null) return text;
  text[section][field] = value;
  return text;
}

function putFromParts(text, parts, value) {
  const [section, field] = parts;
  return put(text, section, field, value);
}

module.exports = putFromParts;

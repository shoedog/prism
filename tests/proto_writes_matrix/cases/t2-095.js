function put(root, query, field, value) {
  const params = new URLSearchParams(query);
  const section = params.get("section");
  if (section === null || root[section] == null) return root;
  root[section][field] = value;
  return root;
}

function putFromParts(root, parts, value) {
  const [section, field] = parts;
  const query = "section=" + encodeURIComponent(section);
  return put(root, query, field, value);
}

module.exports = putFromParts;

function annotateCharacters(text, field = "character") {
  const rows = Array.from({ length: text.length }, () => ({}));
  for (const position in text) {
    rows[position][field] = text[position];
  }
  return rows;
}

module.exports = annotateCharacters;

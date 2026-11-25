function fillHeadings(labels, field = "text") {
  const headings = { title: {}, subtitle: {} };
  function fill(chunks, ...values) {
    for (const [i, value] of values.entries()) {
      const section = chunks[i];
      headings[section][field] = value;
    }
  }
  fill`title${labels.title}subtitle${labels.subtitle}`;
  return headings;
}

module.exports = fillHeadings;

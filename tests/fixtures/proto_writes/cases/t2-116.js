function annotateSamples(rows, samples, field, value) {
  const bytes = Uint8Array.from(samples);
  for (const index of Object.keys(bytes)) {
    if (rows[index] == null) continue;
    rows[index][field] = value;
  }
  return rows;
}

module.exports = annotateSamples;

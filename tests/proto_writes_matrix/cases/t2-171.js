function colorSwatches(index, field = "color") {
  const initial = { color: "gray" };
  const swatches = Array(3).fill(initial);
  if (!Number.isInteger(index) || index < 0 || index >= swatches.length) {
    return swatches;
  }
  swatches[index][field] = "blue";
  return swatches;
}

function colorSwatchesFromText(index) {
  return colorSwatches(Number(index));
}

module.exports = colorSwatchesFromText;

function summarizeCount(name, group = "counts", field = "shown") {
  const counts = Object.assign(Object.create(null), { open: 4, closed: 2 });
  const result = { counts: Object.create(null) };
  ({ [name]: result[group][field] = 0 } = counts);
  return result;
}

module.exports = summarizeCount;

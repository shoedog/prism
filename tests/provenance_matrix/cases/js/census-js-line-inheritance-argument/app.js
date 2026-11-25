function collect(...items) { return items; }
function read() {
  const label = String(7);
  return collect(label, process.env.MODE);
}

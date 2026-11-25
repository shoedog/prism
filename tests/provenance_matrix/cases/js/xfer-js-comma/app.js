function initial(ignored) { return "fixed"; }

function read() {
  const raw = process.argv[2];
  const n = (raw, 7);
  const result = initial(raw);
  return [n, result];
}

function f(stop) {
  let value = process.argv[2];
  if (stop) return value;
  value = process.env.MODE;
  return value;
}

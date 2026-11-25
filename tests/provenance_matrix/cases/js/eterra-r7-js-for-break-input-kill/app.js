function load() {
  let value = process.argv[2];
  for (;;) { value = process.env.MODE; break; }
  return value;
}

function load(fallback) {
  const mode = process.env.MODE ?? fallback;
  return mode;
}

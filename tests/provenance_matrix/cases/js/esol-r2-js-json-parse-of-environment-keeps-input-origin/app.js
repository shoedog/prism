function load() {
  const raw = process.env.SERVICE_OPTIONS ?? '{}';
  const options = JSON.parse(raw);
  return options;
}

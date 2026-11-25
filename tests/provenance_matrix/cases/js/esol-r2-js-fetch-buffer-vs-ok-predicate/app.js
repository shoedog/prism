async function load() {
  const response = await fetch('https://example.test/thumbnail');
  const bytes = await response.arrayBuffer();
  const succeeded = response.ok;
  return [bytes, succeeded];
}

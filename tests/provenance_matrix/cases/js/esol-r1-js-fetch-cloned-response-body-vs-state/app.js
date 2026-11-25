async function load() {
  const response = await fetch('https://example.invalid/catalog');
  const duplicate = response.clone();
  const consumed = duplicate.bodyUsed;
  const payload = await duplicate.text();
  return [payload, consumed];
}

async function read() {
  const response = await fetch('https://example.com');
  const text = await response.text();
  return text;
}

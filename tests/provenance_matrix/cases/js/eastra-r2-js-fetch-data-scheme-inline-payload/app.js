async function edition() {
  const response = await fetch('data:text/plain,preview');
  const text = await response.text();
  return text;
}

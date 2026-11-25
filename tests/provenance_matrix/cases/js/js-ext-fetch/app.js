async function read(url) {
  const response = await fetch(url);
  const payload = await response.json();
  return payload;
}

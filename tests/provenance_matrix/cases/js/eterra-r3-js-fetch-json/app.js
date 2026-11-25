async function run() {
  const response = await fetch('https://example.org');
  const data = await response.json();
  console.log(data);
}

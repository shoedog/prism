async function run() {
  const response = await fetch("https://example.org");
  const body = await response.text();
  console.log(body);
}

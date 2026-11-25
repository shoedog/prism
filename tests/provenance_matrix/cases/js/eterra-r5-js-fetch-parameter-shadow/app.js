async function run(fetch) {
  const value = await fetch("https://example.org");
  console.log(value);
}

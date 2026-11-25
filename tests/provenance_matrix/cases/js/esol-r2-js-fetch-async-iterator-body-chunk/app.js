async function inspect() {
  const response = await fetch('https://example.test/catalog');
  for await (const chunk of response.body) {
    console.log(chunk);
  }
}

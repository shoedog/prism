async function run() {
  for (const fetch of [async () => 'local']) {
    const value = await fetch('https://example.org');
    console.log(value);
  }
}

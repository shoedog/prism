async function versions() {
  const network = await fetch('https://example.invalid/edition');
  const preview = new Response('sample');
  const received = await network.text();
  const local = await preview.text();
  return [received, local];
}

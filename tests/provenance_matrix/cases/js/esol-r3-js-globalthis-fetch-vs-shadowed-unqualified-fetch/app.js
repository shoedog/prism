async function preview() {
  const fetch = async () => 'offline';
  const upstream = await globalThis.fetch('https://example.invalid/catalog');
  const body = await upstream.text();
  const previewText = await fetch();
  return [body, previewText];
}

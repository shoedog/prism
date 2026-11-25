async function firstChunk() {
  const reply = await fetch('https://catalog.example/editions');
  const reader = reply.body.getReader();
  const {value: chunk, done: exhausted} = await reader.read();
  if (exhausted) return null;
  return [chunk, exhausted];
}

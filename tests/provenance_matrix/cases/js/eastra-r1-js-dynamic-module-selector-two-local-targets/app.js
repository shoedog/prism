async function title(rehearsal) {
  const modulePath = rehearsal ? './preview.js' : './draft.js';
  const library = await import(modulePath);
  const result = await library.default.get('/edition');
  return result;
}

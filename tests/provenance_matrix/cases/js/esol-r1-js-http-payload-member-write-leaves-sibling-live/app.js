const axios = require('axios');

async function load() {
  const response = await axios.get('https://example.invalid/catalog');
  const payload = response.data;
  if (!payload || typeof payload !== 'object') return [];
  payload.style = 'preview';
  const style = payload.style;
  const title = payload.title;
  return [style, title];
}

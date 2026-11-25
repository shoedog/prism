const axios = require('axios');
async function labels() {
  const reply = await axios.get('https://catalog.example/labels', {
    transformResponse: [() => 'preview']
  });
  const label = reply.data;
  const revision = reply.headers['x-revision'];
  return [label, revision];
}

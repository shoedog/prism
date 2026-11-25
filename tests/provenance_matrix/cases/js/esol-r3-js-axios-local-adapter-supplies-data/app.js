const axios = require('axios');
const client = axios.create({
  adapter: async config => ({
    config, data: 'local', status: 200, statusText: 'OK', headers: {},
  }),
});
async function load() {
  const { data: value } = await client.get('https://example.invalid/catalog');
  return value;
}

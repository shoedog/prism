const axios = require('axios');
async function read() {
  const client = axios.create({baseURL: 'https://example.com'});
  const response = await client.get('/item');
  return [client, response];
}

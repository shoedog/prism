const axios = require('axios');
async function run() {
  const response = await axios.post('https://example.org', {});
  const payload = response.data;
  console.log(payload);
}

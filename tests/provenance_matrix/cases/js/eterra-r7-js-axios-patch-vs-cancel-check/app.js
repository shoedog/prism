const axios = require('axios');
async function send() {
  const reply = await axios.patch('https://example.test/item', {});
  const cancelled = axios.isCancel(null);
  return [reply, cancelled];
}

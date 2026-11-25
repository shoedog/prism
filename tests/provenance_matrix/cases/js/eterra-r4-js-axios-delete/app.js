const axios = require('axios');
async function run() {
  const response = await axios.delete('https://example.test/item');
  console.log(response);
}

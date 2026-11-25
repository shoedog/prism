const axios = require('./axios');
async function run() {
  const response = await axios.get('https://example.com');
  console.log(response);
}

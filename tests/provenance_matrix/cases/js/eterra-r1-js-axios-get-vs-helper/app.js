const axios = require('axios');
async function run() {
  const response = await axios.get('https://example.com');
  const check = axios.isAxiosError(response);
  console.log(response, check);
}

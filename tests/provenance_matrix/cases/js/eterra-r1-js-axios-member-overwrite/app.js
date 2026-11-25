const axios = require('axios');
axios.get = () => 'fixed';
function run() {
  const response = axios.get('https://example.com');
  console.log(response);
}

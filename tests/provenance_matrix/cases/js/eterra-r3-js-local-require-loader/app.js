function run(require) {
  const axios = require('axios');
  const response = axios.get('https://example.org');
  console.log(response);
}

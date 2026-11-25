let axios = require('axios');
async function run() {
  const response = await axios.get('https://example.org');
  console.log(response);
}
run();
axios = { get: () => 'local' };

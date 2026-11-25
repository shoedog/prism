const client = require('./http-client');
async function run() {
  const response = await client.get('https://example.org');
  console.log(response);
}

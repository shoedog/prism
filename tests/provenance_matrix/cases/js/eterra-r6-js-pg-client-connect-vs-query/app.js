const pg = require('pg');
async function read() {
  const client = new pg.Client();
  const connected = await client.connect();
  const result = await client.query('select 1');
  return [connected, result];
}

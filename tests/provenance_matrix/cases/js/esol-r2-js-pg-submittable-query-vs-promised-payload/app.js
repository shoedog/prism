const pg = require('pg');
async function load() {
  const client = new pg.Client();
  await client.connect();
  const queued = client.query(new pg.Query('select 1'));
  const result = await client.query('select 2');
  await client.end();
  return [queued, result];
}

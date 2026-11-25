const { Client } = require('pg');
async function run() {
  const client = new Client();
  const rows = await client.query('select 1');
  console.log(rows);
}

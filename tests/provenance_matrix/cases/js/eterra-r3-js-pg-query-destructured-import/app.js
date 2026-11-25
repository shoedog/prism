const { Client: DBClient } = require('pg');
async function run() {
  const client = new DBClient();
  const rows = await client.query('select 1');
  console.log(rows);
}

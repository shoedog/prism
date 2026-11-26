const { Pool } = require('pg');
const pool = new Pool();
async function run() {
  const result = await pool.query('select 1');
  console.log(result.rowCount);
}

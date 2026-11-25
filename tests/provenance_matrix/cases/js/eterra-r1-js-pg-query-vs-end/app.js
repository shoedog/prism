const pg = require('pg');
async function run() {
  const pool = new pg.Pool();
  const result = await pool.query('select 1');
  const closed = await pool.end();
  console.log(result, closed);
}

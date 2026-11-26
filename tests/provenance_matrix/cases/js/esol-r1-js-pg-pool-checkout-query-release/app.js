const pg = require('pg');

async function load() {
  const pool = new pg.Pool();
  const connection = await pool.connect();
  let result;
  let released;
  try {
    result = await connection.query('select 1 as item');
  } finally {
    released = connection.release();
    await pool.end();
  }
  return [result, released];
}

import * as pg from 'pg';
async function read() {
  const pool = new pg.Pool();
  const result = await pool.query('select 1');
  return result;
}

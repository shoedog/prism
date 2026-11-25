const { Pool } = require('pg');
async function load() {
  const pool = new Pool();
  const result = await pool.query({ text: 'select 1', values: [] });
  return result;
}

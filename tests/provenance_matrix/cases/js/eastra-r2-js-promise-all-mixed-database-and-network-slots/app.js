const pg = require('pg');
const axios = require('axios');
const pool = new pg.Pool();
async function snapshot() {
  const [remote, records] = await Promise.all([
    axios.get('https://example.invalid/edition'),
    pool.query('select title from books')
  ]);
  const body = remote.data;
  const rows = records.rows;
  return [body, rows];
}

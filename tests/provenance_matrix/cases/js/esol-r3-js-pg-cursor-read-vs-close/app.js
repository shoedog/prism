const { Pool } = require('pg');
const Cursor = require('pg-cursor');
const pool = new Pool();
async function load() {
  const client = await pool.connect();
  try {
    const cursor = client.query(new Cursor('select label from catalog'));
    const { rows: records } = await cursor.read(10);
    const closed = await cursor.close();
    return [records, closed];
  } finally {
    client.release();
  }
}

const pg = require('pg');
async function lookup() {
  const pool = new pg.Pool();
  try {
    const result = await pool.query({
      text: 'select label from catalog',
      rowMode: 'array',
    });
    if (result.rows.length === 0) return [];
    const [[title]] = result.rows;
    const [{ name: column }] = result.fields;
    return [title, column];
  } finally {
    await pool.end();
  }
}

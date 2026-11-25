const pg = require('pg');
async function labels() {
  const pool = new pg.Pool();
  try {
    const text = 'select title::text as title from editions where id = 1 and title is not null';
    const live = await pool.query(text);
    const masked = await pool.query({ text, types: { getTypeParser: () => () => 'sample' } });
    if (!live.rows.length || !masked.rows.length) return;
    const actual = live.rows[0].title;
    const replacement = masked.rows[0].title;
    return [actual, replacement];
  } finally {
    await pool.end();
  }
}

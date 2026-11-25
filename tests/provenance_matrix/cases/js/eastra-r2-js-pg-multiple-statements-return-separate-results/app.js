const pg = require('pg');
const pool = new pg.Pool();
async function editions() {
  const [authors, books] = await pool.query(
    'select name from authors; select title from books'
  );
  const first = authors.rows;
  const second = books.rows;
  return [first, second];
}

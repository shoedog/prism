const pg = require('pg');
const other = pg.Pool.prototype;
Object.defineProperty(other, 'query', {
  value: async () => ({ rows: ['sample'] }),
});
async function preview() {
  const pool = new pg.Pool();
  const result = await pool.query('select title from editions');
  const label = result.rows[0];
  return label;
}

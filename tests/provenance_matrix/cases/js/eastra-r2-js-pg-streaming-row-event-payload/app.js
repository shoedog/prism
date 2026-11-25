const pg = require('pg');
async function stream(emit) {
  const client = new pg.Client();
  await client.connect();
  const query = new pg.Query('select title from books');
  query.on('row', (row, result) => {
    const title = row.title;
    const column = result.fields[0].name;
    emit(title, column);
  });
  query.on('end', () => client.end());
  client.query(query);
}

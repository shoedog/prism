const pg = require('pg');

function load() {
  const client = new pg.Client();
  client.connect();
  client.query('select 1 as item', (failure, result) => {
    console.log(failure, result);
    void client.end();
  });
}

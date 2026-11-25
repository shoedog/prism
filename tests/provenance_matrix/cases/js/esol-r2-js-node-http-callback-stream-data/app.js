const http = require('node:http');
function inspect() {
  http.get('http://example.test/catalog', (response) => {
    response.on('data', (chunk) => {
      console.log(chunk);
    });
  });
}

async function run() {
  const axios = await import('axios');
  const response = await axios.default.get('https://example.com');
  console.log(response);
}

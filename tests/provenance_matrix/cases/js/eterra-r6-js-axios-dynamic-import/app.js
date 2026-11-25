async function read() {
  const axios = await import('axios');
  const response = await axios.default.get('https://example.com');
  return response;
}

import axios from 'axios';
export async function load() {
  const { data: payload, request: transport } = await axios.get('https://example.test/catalog');
  return [payload, transport];
}

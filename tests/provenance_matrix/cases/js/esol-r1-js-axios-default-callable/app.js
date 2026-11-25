import axios from 'axios';

export async function load() {
  const { data: payload } = await axios({ url: 'https://example.invalid/catalog' });
  return payload;
}

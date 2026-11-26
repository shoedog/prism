const courier = {
  get: (url: string) => fetch(url),
  post: (url: string) => fetch(url, { method: 'POST' }),
};
export default courier;

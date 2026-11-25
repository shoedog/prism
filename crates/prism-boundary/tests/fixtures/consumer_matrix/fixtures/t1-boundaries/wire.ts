const wire = {
  get: (path: string) => fetch(path),
  post: (path: string) => fetch(path, { method: 'POST' }),
};
export default wire;
